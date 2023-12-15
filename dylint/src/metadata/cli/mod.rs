//! This module borrows an idea from [Marker]: to use `cargo fetch` to download a package into
//! Cargo's cache. More specifically, this module creates a "dummy" project with a specified package
//! as a dependency, and then calls `cargo fetch` to download the project's dependencies into
//! Cargo's cache.
//!
//! There is a complication, however. Dylint does not require a workspace metadata entry to specify
//! a lint library's package name. But the above idea, as applied in Marker, requires the package
//! name.
//!
//! To work around this problem, this module creates a "dummy" dependency with a random name and
//! "injects" it into each subdirectory of the relevant checkouts directory. If Cargo finds the
//! dummy dependency in one of those subdirectories, then that subdirectory must have been updated
//! by `cargo fetch`. On the other hand, if Cargo finds the dummy dependency in a completely new
//! subdirectory, then that subdirectory must have been created by `cargo fetch`.
//!
//! [Marker]: https://github.com/rust-marker/marker

use anyhow::{anyhow, bail, ensure, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use dylint_internal::{packaging::isolate, CommandExt};
use home::cargo_home;
use semver::Version;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, Builder, TempDir};
use url::Url;

mod string_or_vec;
use string_or_vec::StringOrVec;

mod util;
use util::{short_hash, CanonicalUrl};

struct NamedTempDir(PathBuf);

impl Drop for NamedTempDir {
    fn drop(&mut self) {
        remove_dir_all(&self.0).unwrap_or_default();
    }
}

// smoelius: Use `include!` so that `DetailedTomlDependency`'s fields are visible without having to
// make them all `pub`.
include!("detailed_toml_dependency.rs");

pub struct Config;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageId {
    name: String,
    version: Version,
    source_id: String,
}

pub type SourceId = String;

impl Config {
    #[allow(clippy::unnecessary_wraps)]
    pub const fn default() -> Result<Self> {
        Ok(Self)
    }
}

impl PackageId {
    #[allow(clippy::unnecessary_wraps)]
    pub const fn new(name: String, version: Version, source_id: SourceId) -> Result<Self> {
        Ok(Self {
            name,
            version,
            source_id,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn dependency_source_id_and_root(
    _opts: &crate::Dylint,
    metadata: &Metadata,
    _config: &Config,
    details: &DetailedTomlDependency,
) -> Result<(SourceId, PathBuf)> {
    if let Some(url) = &details.git {
        ensure!(
            details.path.is_none(),
            "A dependency cannot have both git and path entries"
        );
        let source_id = git_source_id(url, details)?;
        let root = git_dependency_root(url, details)?;
        Ok((source_id, root))
    } else if let Some(path) = &details.path {
        let source_id = String::new();
        let root = metadata
            .workspace_root
            .join(path)
            .as_std_path()
            .to_path_buf();
        Ok((source_id, root))
    } else {
        bail!("Only git and path entries are supported")
    }
}

fn git_source_id(url: &str, details: &DetailedTomlDependency) -> Result<String> {
    #[derive(Serialize)]
    struct GitReference<'a> {
        url: &'a str,
        branch: Option<&'a str>,
        tag: Option<&'a str>,
        rev: Option<&'a str>,
    }
    let json = serde_json::to_string(&GitReference {
        url,
        branch: details.branch.as_deref(),
        tag: details.tag.as_deref(),
        rev: details.rev.as_deref(),
    })?;
    Ok(json)
}

fn git_dependency_root(url: &str, details: &DetailedTomlDependency) -> Result<PathBuf> {
    let dependency = create_dummy_dependency()?;
    let filename = dependency
        .path()
        .file_name()
        .ok_or_else(|| anyhow!("Could not get file name"))?;
    let dep_name = filename.to_string_lossy();

    let package = create_dummy_package(&dep_name, details)?;

    let cargo_home = cargo_home().with_context(|| "Could not determine `CARGO_HOME`")?;
    let ident = ident(url)?;
    let checkout_path = cargo_home.join("git/checkouts").join(ident);

    // smoelius: `checkout_path` might not exist, e.g., if the url has never been cloned.
    let injected_dependencies = if checkout_path
        .try_exists()
        .with_context(|| format!("Could not determine whether {checkout_path:?} exists"))?
    {
        inject_dummy_dependencies(dependency.path(), &dep_name, &checkout_path)?
    } else {
        BTreeMap::new()
    };

    cargo_fetch(package.path())?;

    // smoelius: `cargo metadata` will fail if `cargo fetch` had to create a new checkouts
    // subdirectory.
    let metadata = cargo_metadata(package.path()).ok();

    let path = find_accessed_subdir(
        &dep_name,
        &checkout_path,
        &injected_dependencies,
        metadata.as_ref(),
    )?;

    Ok(path.to_path_buf())
}

/// Creates a dummy dependency in a temporary directory, and returns the temporary directory if
/// everything was successful.
fn create_dummy_dependency() -> Result<TempDir> {
    let tempdir = Builder::new()
        .prefix("tmp")
        .tempdir()
        .with_context(|| "Could not create temporary directory")?;

    dylint_internal::cargo::init("dummy dependency")
        .quiet(true)
        .stable(true)
        .build()
        .current_dir(&tempdir)
        .args(["--lib", "--vcs=none"])
        .success()?;

    isolate(tempdir.path())?;

    Ok(tempdir)
}

/// Creates a dummy package in a temporary directory, and returns the temporary directory if
/// everything was successful.
fn create_dummy_package(dep_name: &str, details: &DetailedTomlDependency) -> Result<TempDir> {
    let tempdir = tempdir().with_context(|| "Could not create temporary directory")?;

    let manifest_contents = manifest_contents(dep_name, details)?;
    let manifest_path = tempdir.path().join("Cargo.toml");
    write(&manifest_path, manifest_contents)
        .with_context(|| format!("Could not write to {manifest_path:?}"))?;

    let src_path = tempdir.path().join("src");

    create_dir_all(&src_path)
        .with_context(|| format!("`create_dir_all` failed for `{src_path:?}`"))?;

    let main_rs_path = src_path.join("main.rs");
    write(&main_rs_path, "fn main() {}")
        .with_context(|| format!("Could not write to {main_rs_path:?}"))?;

    Ok(tempdir)
}

fn manifest_contents(dep_name: &str, details: &DetailedTomlDependency) -> Result<String> {
    let details = toml::to_string(details)?;

    Ok(format!(
        r#"
[package]
name = "dummy-package"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies.{dep_name}]
{details}
"#
    ))
}

fn inject_dummy_dependencies(
    dep_path: &Path,
    dep_name: &str,
    checkout_path: &Path,
) -> Result<BTreeMap<OsString, NamedTempDir>> {
    let mut injected_dependencies = BTreeMap::new();
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    for_each_subdir(checkout_path, |subdir, path| {
        injected_dependencies.insert(subdir.to_owned(), NamedTempDir(path.join(dep_name)));
        fs_extra::dir::copy(dep_path, path, &fs_extra::dir::CopyOptions::default())?;
        Ok(())
    })?;
    Ok(injected_dependencies)
}

fn cargo_fetch(path: &Path) -> Result<()> {
    // smoelius: `cargo fetch` could fail, e.g., if a new checkouts subdirectory had to be created.
    // But the command should still be executed.
    let _output = dylint_internal::cargo::fetch("dummy package")
        .quiet(true)
        .stable(true)
        .build()
        .args([
            "--manifest-path",
            &path.join("Cargo.toml").to_string_lossy(),
        ])
        .logged_output()?;
    Ok(())
}

fn cargo_metadata(path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .current_dir(path)
        .exec()
        .map_err(Into::into)
}

// smoelius: `ident` is based on the function of the same name at:
// https://github.com/rust-lang/cargo/blob/1a498b6c1c119a79d677553862bffae96b97ad7f/src/cargo/sources/git/source.rs#L136-L147
#[allow(clippy::manual_next_back)]
#[cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]
fn ident(url: &str) -> Result<String> {
    let url = Url::parse(url)?;

    let canonical_url = CanonicalUrl::new(&url)?;

    let ident = canonical_url
        .raw_canonicalized_url()
        .path_segments()
        .and_then(|s| s.rev().next())
        .unwrap_or("");

    let ident = if ident.is_empty() { "_empty" } else { ident };

    Ok(format!("{}-{}", ident, short_hash(&canonical_url)))
}

fn find_accessed_subdir<'a>(
    dep_name: &str,
    checkout_path: &Path,
    injected_dependencies: &BTreeMap<OsString, NamedTempDir>,
    metadata: Option<&'a Metadata>,
) -> Result<Cow<'a, Path>> {
    let mut accessed = metadata
        .map_or::<&[_], _>(&[], |metadata| &metadata.packages)
        .iter()
        .map(|package| {
            if package.name == dep_name {
                let parent = package
                    .manifest_path
                    .parent()
                    .ok_or_else(|| anyhow!("Could not get parent directory"))?;
                let grandparent = parent
                    .parent()
                    .ok_or_else(|| anyhow!("Could not get grandparent directory"))?;
                Ok(Some(Cow::Borrowed(grandparent.as_std_path())))
            } else {
                Ok(None)
            }
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>>>()?;

    // smoelius: If no subdirectories were accessed, then some checkouts subdirectory should have
    // been created.
    if accessed.is_empty() {
        for_each_subdir(checkout_path, |subdir, path| {
            if injected_dependencies.get(subdir).is_none() {
                accessed.push(Cow::Owned(path.to_path_buf()));
            }
            Ok(())
        })?;
    }

    ensure!(
        accessed.len() <= 1,
        "Multiple subdirectories were accessed: {:#?}",
        accessed
    );

    accessed
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not determined accessed subdirectory"))
}

fn for_each_subdir(
    checkout_path: &Path,
    mut f: impl FnMut(&OsStr, &Path) -> Result<()>,
) -> Result<()> {
    for entry in read_dir(checkout_path)
        .with_context(|| format!("`read_dir` failed for {checkout_path:?}"))?
    {
        let entry = entry.with_context(|| format!("`read_dir` failed for {checkout_path:?}"))?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Could not get file name"))?;
        if !path.is_dir() {
            continue;
        }
        f(file_name, &path)?;
    }
    Ok(())
}
