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

use crate::opts;
use anyhow::{Context, Result, anyhow, bail, ensure};
use cargo_metadata::{Metadata, MetadataCommand};
use cargo_util_schemas::manifest::TomlDetailedDependency;
use dylint_internal::{CommandExt, home::cargo_home, packaging::isolate};
use semver::Version;
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, read_dir, remove_dir_all, write},
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir, tempdir};
use url::Url;

mod util;
use util::{CanonicalUrl, short_hash};

struct NamedTempDir(PathBuf);

impl Drop for NamedTempDir {
    fn drop(&mut self) {
        remove_dir_all(&self.0).unwrap_or_default();
    }
}

pub struct GlobalContext;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageId {
    name: String,
    version: Version,
    source_id: String,
}

pub type SourceId = String;

impl GlobalContext {
    #[allow(clippy::unnecessary_wraps)]
    pub const fn default() -> Result<Self> {
        Ok(Self)
    }
}

impl PackageId {
    #[allow(clippy::unnecessary_wraps)]
    pub const fn new(name: String, version: Version, source_id: SourceId) -> Self {
        Self {
            name,
            version,
            source_id,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn dependency_source_id_and_root(
    _opts: &opts::Dylint,
    metadata: &Metadata,
    _gctx: &GlobalContext,
    details: &TomlDetailedDependency,
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

fn git_source_id(url: &str, details: &TomlDetailedDependency) -> Result<String> {
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

fn git_dependency_root(url: &str, details: &TomlDetailedDependency) -> Result<PathBuf> {
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

    let mut errors = Vec::new();
    // smoelius: It should take at most two attempts to find the git dependency root. The first
    // attempt may fail because a new checkouts subdirectory had to be created. But the second
    // attempt should then succeed.
    for _ in [false, true] {
        // smoelius: `checkout_path` might not exist, e.g., if the url has never been cloned.
        let injected_dependencies = if checkout_path.try_exists().with_context(|| {
            format!(
                "Could not determine whether `{}` exists",
                checkout_path.display()
            )
        })? {
            inject_dummy_dependencies(dependency.path(), &dep_name, &checkout_path)?
        } else {
            BTreeMap::new()
        };

        let output = cargo_fetch(package.path())?;

        // smoelius: `cargo metadata` will fail if `cargo fetch` had to create a new checkouts
        // subdirectory.
        let metadata = cargo_metadata(package.path()).ok();

        match find_accessed_subdir(
            &dep_name,
            &checkout_path,
            &injected_dependencies,
            metadata.as_ref(),
        ) {
            Ok(path) => {
                return Ok(path.to_path_buf());
            }
            Err(error) => {
                let s = if output.status.success() {
                    error.to_string()
                } else {
                    format!(
                        "{:?}",
                        Result::<PathBuf>::Err(error).with_context(|| {
                            format!(
                                "fetching packages failed\nstdout: {:?}\nstderr: {:?}",
                                String::from_utf8(output.stdout).unwrap_or_default(),
                                dummy_dependency_free_suffix(
                                    &dep_name,
                                    &String::from_utf8(output.stderr).unwrap_or_default()
                                )
                            )
                        })
                    )
                };
                errors.push(s);
            }
        }
    }

    // smoelius: If we get here, it should be because `find_accessed_subdir` failed twice.
    debug_assert!(errors.len() >= 2);

    Err(anyhow!("Could not find git dependency root: {errors:#?}"))
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
fn create_dummy_package(dep_name: &str, details: &TomlDetailedDependency) -> Result<TempDir> {
    let tempdir = tempdir().with_context(|| "Could not create temporary directory")?;

    let manifest_contents = manifest_contents(dep_name, details)?;
    let manifest_path = tempdir.path().join("Cargo.toml");
    write(&manifest_path, manifest_contents)
        .with_context(|| format!("Could not write to `{}`", manifest_path.display()))?;

    let src_path = tempdir.path().join("src");

    create_dir_all(&src_path)
        .with_context(|| format!("`create_dir_all` failed for `{}`", src_path.display()))?;

    let main_rs_path = src_path.join("main.rs");
    write(&main_rs_path, "fn main() {}")
        .with_context(|| format!("Could not write to `{}`", main_rs_path.display()))?;

    Ok(tempdir)
}

fn manifest_contents(dep_name: &str, details: &TomlDetailedDependency) -> Result<String> {
    let details = toml::to_string(details)?;

    Ok(format!(
        r#"
[package]
name = "dummy-package"
version = "0.1.0"
edition = "2024"
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

fn cargo_fetch(path: &Path) -> Result<std::process::Output> {
    // smoelius: `cargo fetch` could fail, e.g., if a new checkouts subdirectory had to be created.
    // But the command should still be executed.
    // smoelius: Since stdout and stderr are captured, there is no need to use `.quiet(true)`.
    // smoelius: We still want to hide the "Fetching ..." message, though.
    dylint_internal::cargo::fetch("dummy package")
        .quiet(dylint_internal::cargo::Quiet::MESSAGE)
        .stable(true)
        .build()
        .args([
            "--manifest-path",
            &path.join("Cargo.toml").to_string_lossy(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .logged_output(false)
}

fn cargo_metadata(path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .cargo_path(dylint_internal::cargo::stable_cargo_path())
        .current_dir(path)
        .exec()
        .map_err(Into::into)
}

// smoelius: `ident` is based on the function of the same name at:
// https://github.com/rust-lang/cargo/blob/1a498b6c1c119a79d677553862bffae96b97ad7f/src/cargo/sources/git/source.rs#L136-L147
#[allow(clippy::manual_next_back)]
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
            if package.name.as_str() == dep_name {
                let parent = package
                    .manifest_path
                    .parent()
                    .ok_or_else(|| anyhow!("Could not get parent directory"))?;
                let grandparent = parent
                    .parent()
                    .ok_or_else(|| anyhow!("Could not get grandparent directory"))?;
                #[cfg(debug_assertions)]
                eprintln!(
                    "{}:{:?}: accessed: {grandparent:?}",
                    std::process::id(),
                    std::thread::current().id()
                );
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
                #[cfg(debug_assertions)]
                eprintln!(
                    "{}:{:?}: pushing `{}`",
                    std::process::id(),
                    std::thread::current().id(),
                    path.display()
                );
                accessed.push(Cow::Owned(path.to_path_buf()));
            }
            Ok(())
        })?;
    }

    ensure!(
        accessed.len() <= 1,
        "Multiple subdirectories were accessed: {accessed:#?}"
    );

    accessed
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not determine accessed subdirectory"))
}

fn for_each_subdir(
    checkout_path: &Path,
    mut f: impl FnMut(&OsStr, &Path) -> Result<()>,
) -> Result<()> {
    for entry in read_dir(checkout_path)
        .with_context(|| format!("`read_dir` failed for `{}`", checkout_path.display()))?
    {
        let entry = entry
            .with_context(|| format!("`read_dir` failed for `{}`", checkout_path.display()))?;
        let file_name = entry.file_name();
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        f(&file_name, &path)?;
    }
    Ok(())
}

fn dummy_dependency_free_suffix(dep_name: &str, s: &str) -> String {
    // smoelius: The `{..}` are a hack to prevent triggering `misleading_variable_name`.
    let lines = { s.split_inclusive('\n') };
    if let Some(i) = lines.clone().rev().position(|line| line.contains(dep_name)) {
        let n = lines.clone().count();
        lines.skip(n - i).collect()
    } else {
        s.to_owned()
    }
}
