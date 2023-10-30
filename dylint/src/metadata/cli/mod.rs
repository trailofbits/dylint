//! This module borrows an idea from [Marker]: to use `cargo fetch` to download a package into
//! Cargo's cache. More specifically, this module creates a "dummy" project with a specified package
//! as a dependency, and then calls `cargo fetch` to download the project's dependencies into
//! Cargo's cache.
//!
//! There is a complication, however. Dylint does not require a workspace metadata entry to specify
//! a lint library's package name. But the above idea, as applied in Marker, requires the package
//! name.
//!
//! To work around this problem, this module calls `cargo fetch` expecting it to fail, but to still
//! download the package. The relevant checkout directory in Cargo's cache is then walked to see
//! which subdirectory was accessed, as that should be the one containing the package.
//!
//! [Marker]: https://github.com/rust-marker/marker

use anyhow::{anyhow, bail, ensure, Context, Result};
use cargo_metadata::Metadata;
use home::cargo_home;
use semver::Version;
use std::{
    ffi::{OsStr, OsString},
    fs::{create_dir_all, metadata, read_dir, read_to_string, write},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};
use tempfile::{tempdir, TempDir};
use url::Url;

mod string_or_vec;
use string_or_vec::StringOrVec;

mod util;
use util::{short_hash, CanonicalUrl};

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
    let tempdir = create_dummy_package(details)?;

    let cargo_home = cargo_home().with_context(|| "Could not determine `CARGO_HOME`")?;
    let ident = ident(url)?;
    let checkout_path = cargo_home.join("git/checkouts").join(ident);

    // smoelius: Under some circumstances, a file must be modified before its access time is
    // updated. See the following StackExchange answer for some discussion:
    // https://unix.stackexchange.com/a/581253
    let atimes = touch_subdirs(&checkout_path)?;

    cargo_fetch(tempdir.path())?;

    let subdir = find_accessed_subdir(&checkout_path, &atimes)?;

    Ok(checkout_path.join(subdir))
}

/// Creates a dummy package in a temporary directory, and returns the temporary directory if
/// everything was successful.
fn create_dummy_package(details: &DetailedTomlDependency) -> Result<TempDir> {
    let tempdir = tempdir().with_context(|| "Could not create temporary directory")?;

    let manifest_contents = manifest_contents(details)?;
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

fn manifest_contents(details: &DetailedTomlDependency) -> Result<String> {
    let details = toml::to_string(details)?;

    Ok(format!(
        r#"
[package]
name = "dummy-package"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies.dummy-dependency]
{details}
"#
    ))
}

fn cargo_fetch(path: &Path) -> Result<()> {
    // smoelius: We expect `cargo fetch` to fail, but the command should still be executed.
    let mut command = Command::new("cargo");
    command.args([
        "fetch",
        "--manifest-path",
        &path.join("Cargo.toml").to_string_lossy(),
    ]);
    let _output = command
        .output()
        .with_context(|| format!("Could not get output of `{command:?}`"))?;
    Ok(())
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

fn touch_subdirs(checkout_path: &Path) -> Result<BTreeMap<OsString, SystemTime>> {
    let mut map = BTreeMap::new();
    for_each_head(checkout_path, |subdir, head| {
        touch(head)?;
        let atime = atime(head)?;
        map.insert(subdir.to_owned(), atime);
        Ok(())
    })?;
    Ok(map)
}

fn find_accessed_subdir(
    checkout_path: &Path,
    atimes: &BTreeMap<OsString, SystemTime>,
) -> Result<OsString> {
    let mut accessed = Vec::new();
    for_each_head(checkout_path, |subdir, head| {
        let atime = atime(head)?;
        if atimes.get(subdir) != Some(&atime) {
            accessed.push(subdir.to_owned());
        }
        Ok(())
    })?;
    ensure!(accessed.len() <= 1, "Multiple subdirectories were accessed");
    accessed
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Could not determined accessed subdirectory"))
}

fn for_each_head(
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
        let head = path.join(".git/HEAD");
        f(file_name, &head)?;
    }
    Ok(())
}

/// Update a file's modification time by reading and writing its contents.
fn touch(path: &Path) -> Result<()> {
    let contents = read_to_string(path).with_context(|| format!("Could not read from {path:?}"))?;
    write(path, contents).with_context(|| format!("Could not write to {path:?}"))?;
    Ok(())
}

fn atime(path: &Path) -> Result<SystemTime> {
    let metadata = metadata(path).with_context(|| format!("Could not get metadata of {path:?}"))?;
    let atime = metadata
        .accessed()
        .with_context(|| format!("Could not get access time of {path:?}"))?;
    Ok(atime)
}
