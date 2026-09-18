use anyhow::{Result, anyhow, bail, ensure};
use dylint_internal::{CommandExt, rustup::SanitizeEnvironment};
use quote::ToTokens;
use serde::Deserialize;
use std::{
    env::consts,
    fs::{File, OpenOptions, create_dir, write},
    io::Write,
    path::PathBuf,
    process::Command,
};
use syn::spanned::Spanned;
use tempfile::TempDir;

#[derive(Deserialize)]
pub struct Pattern<T, U> {
    pub pattern: T,
    pub predicate: Option<U>,
    pub callback: Option<U>,
    pub dependencies: Option<String>,
    pub reason: Option<String>,
}

pub type UncompiledPattern = Pattern<String, String>;

pub type CompiledPattern = Pattern<match_hir::Pattern, CallbackDir>;

impl UncompiledPattern {
    /// # Panics
    ///
    /// Panics if a pattern cannot be parsed, or if a predicate or callback cannot be compiled.
    pub fn compile(self) -> CompiledPattern {
        let Pattern {
            pattern,
            predicate,
            callback,
            dependencies,
            reason,
        } = self;
        let pattern = pattern.parse::<match_hir::Pattern>().unwrap();
        let predicate =
            predicate.map(|predicate| compile(&predicate, dependencies.as_deref(), true).unwrap());
        let callback =
            callback.map(|callback| compile(&callback, dependencies.as_deref(), false).unwrap());
        Pattern {
            pattern,
            predicate,
            callback,
            dependencies: None,
            reason,
        }
    }
}

const RUST_TOOLCHAIN: &str = include_str!("../rust-toolchain");
const CLIPPY_UTILS_REV: &str = include_str!(concat!(env!("OUT_DIR"), "/clippy_utils_rev.txt"));

pub struct CallbackDir {
    tempdir: TempDir,
}

fn compile(callback: &str, dependencies: Option<&str>, is_predicate: bool) -> Result<CallbackDir> {
    let (ident_types, body) = parse_callback(callback)?;
    let output = if is_predicate { " -> bool" } else { "" };

    let tempdir = TempDir::new()?;

    write(tempdir.path().join("rust-toolchain"), RUST_TOOLCHAIN)?;

    write(
        tempdir.path().join("Cargo.toml"),
        format!(
            r#"
[package]
name = "callback"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
clippy_utils = {{ git = "https://github.com/rust-lang/rust-clippy", rev = "{CLIPPY_UTILS_REV}" }}
{}
"#,
            dependencies.unwrap_or_default().trim_start()
        ),
    )?;

    create_dir(tempdir.path().join("src"))?;

    let mut lib_rs = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(tempdir.path().join("src/lib.rs"))?;
    writeln!(
        &mut lib_rs,
        r#"
#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_lint_defs;

#[allow(clippy::wildcard_imports)]
use rustc_hir::*;

use rustc_lint::LateContext;

rustc_lint_defs::declare_lint! {{
    DISALLOWED_PATTERN,
    Warn,
    "a disallowed pattern"
}}"#
    )?;
    write_callback(&mut lib_rs, &ident_types, output, body)?;

    // smoelius: Ideally, this code would be combined with the code for building library packages.
    Command::new("cargo")
        .sanitize_environment()
        .env_remove(dylint_internal::env::RUSTFLAGS)
        .current_dir(&tempdir)
        .args(["build", "--quiet", "--release"])
        .success()?;

    Ok(CallbackDir { tempdir })
}

#[derive(Debug)]
struct IdentType {
    ident: String,
    ty: syn::Type,
}

fn parse_callback(callback: &str) -> Result<(Vec<IdentType>, &str)> {
    let closure = syn::parse_str::<syn::ExprClosure>(callback)?;
    let syn::ExprClosure {
        attrs,
        lifetimes,
        constness,
        movability,
        asyncness,
        capture,
        or1_token: _,
        inputs,
        or2_token: _,
        output,
        body,
    } = closure;
    ensure!(attrs.is_empty());
    ensure!(lifetimes.is_none());
    ensure!(constness.is_none());
    ensure!(movability.is_none());
    ensure!(asyncness.is_none());
    ensure!(capture.is_none());
    let ident_types = parse_inputs(inputs.iter())?;
    ensure!(matches!(output, syn::ReturnType::Default));
    let body = &callback[body.span().byte_range()];
    Ok((ident_types, body))
}

fn parse_inputs<'a>(inputs: impl Iterator<Item = &'a syn::Pat>) -> Result<Vec<IdentType>> {
    let mut ident_types = Vec::new();
    for input in inputs {
        let syn::Pat::Type(syn::PatType {
            attrs: attrs_type,
            pat,
            colon_token: _,
            ty,
        }) = input
        else {
            bail!("unexpected pattern: {}", input.to_token_stream());
        };
        if !attrs_type.is_empty() {
            bail!("pattern cannot have attributes: {:?}", attrs_type);
        }
        let ident = match &**pat {
            syn::Pat::Ident(syn::PatIdent {
                attrs: attrs_pat,
                by_ref,
                mutability,
                ident,
                subpat,
            }) if attrs_pat.is_empty()
                && by_ref.is_none()
                && mutability.is_none()
                && subpat.is_none() =>
            {
                ident.to_string()
            }
            syn::Pat::Wild(syn::PatWild {
                attrs: attrs_pat,
                underscore_token: _,
            }) if attrs_pat.is_empty() => String::from("_"),
            _ => {
                bail!("unexpected pattern: {}", input.to_token_stream());
            }
        };
        ident_types.push(IdentType {
            ident,
            ty: *ty.clone(),
        })
    }
    Ok(ident_types)
}

fn write_callback(
    lib_rs: &mut File,
    ident_types: &[IdentType],
    output: &str,
    body: &str,
) -> Result<()> {
    let Some(IdentType {
        ident: cx_ident,
        ty: cx_ty,
    }) = ident_types.first()
    else {
        bail!("callback's first argument should be a `&LateContext<'_>`");
    };

    ensure!(cx_ident != "hir_ids", "`hir_ids` is reserved");

    writeln!(
        lib_rs,
        r"
#[unsafe(no_mangle)]
pub fn callback<'tcx>("
    )?;
    writeln!(lib_rs, "    {cx_ident}: {},", cx_ty.to_token_stream())?;
    writeln!(lib_rs, "    hir_ids: &[HirId],")?;
    writeln!(lib_rs, r"){output} {{")?;
    writeln!(
        lib_rs,
        r#"    assert_eq!(1 + hir_ids.len(), {}, "expected {{}} closure arguments; got {0}", 1 + hir_ids.len());"#,
        ident_types.len()
    )?;
    for (i, IdentType { ident, ty }) in ident_types.iter().skip(1).enumerate() {
        let node_ty = parse_input_ty(ty)?;
        writeln!(
            lib_rs,
            "    let {ident} = {cx_ident}.tcx.hir_node(hir_ids[{i}]).expect_{node_ty}();"
        )?;
    }
    writeln!(lib_rs, "  {body}")?;
    writeln!(lib_rs, "}}")?;

    Ok(())
}

fn parse_input_ty(ty: &syn::Type) -> Result<String> {
    if let syn::Type::Reference(syn::TypeReference {
        and_token: _,
        lifetime: _,
        mutability,
        elem,
    }) = ty
        && mutability.is_none()
        && let syn::Type::Path(syn::TypePath { qself, path }) = &**elem
        && qself.is_none()
        && let syn::Path {
            leading_colon,
            segments,
        } = path
        && leading_colon.is_none()
        && segments.len() == 1
    {
        let syn::PathSegment {
            ident,
            arguments: _,
        } = &segments[0];
        Ok(ident.to_string().to_lowercase())
    } else {
        Err(anyhow!("unexpected type: {}", ty.to_token_stream()))
    }
}

impl CallbackDir {
    pub fn lib_path(&self) -> PathBuf {
        self.tempdir.path().join(format!(
            "target/release/{}callback{}",
            consts::DLL_PREFIX,
            consts::DLL_SUFFIX
        ))
    }
}
