use crate::{
    env, library_filename_with_toolchain, library_plain_filename, parse_plain_path,
    rustup::active_toolchain,
};
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use std::{
    env::current_dir,
    ffi::OsStr,
    fs::{File, copy, metadata},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    sync::LazyLock,
};

// smoelius: The functions in this file use both `name` and `lib_name` as argument names. The
// intuition is as follows:
// - `name` could be a package name and may contain occurrences of `-` that must be replaced with
//   `_` (see, e.g., `library_plain_filename`).
// - `lib_name` is a library name and therefore should contain no occurrences of `-`.

pub fn dylint_libs(name: &str, toolchain: &str) -> Result<String> {
    let current_dir = current_dir().with_context(|| "Could not get current directory")?;
    let path_with_toolchain = debug_path_with_toolchain(&current_dir, name, toolchain)?;

    let paths = vec![path_with_toolchain];
    serde_json::to_string(&paths).map_err(Into::into)
}

pub fn debug_path_with_active_toolchain(dir: &Path, name: &str) -> Result<PathBuf> {
    let toolchain = active_toolchain(dir)?;

    debug_path_with_toolchain(dir, name, &toolchain)
}

pub fn debug_path_with_toolchain(dir: &Path, name: &str, toolchain: &str) -> Result<PathBuf> {
    let metadata = crate::cargo::metadata(dir)?;

    if env::enabled(env::DYLINT_LINK_ENABLE_COPY_LIBRARY) {
        let debug_dir = metadata
            .build_directory
            .as_ref()
            .unwrap_or(&metadata.target_directory)
            .join("debug");
        let path_with_toolchain = debug_dir.join(library_filename_with_toolchain(name, toolchain));
        Ok(path_with_toolchain.into_std_path_buf())
    } else {
        let debug_dir = metadata.target_directory.join("debug");
        let plain_path = debug_dir.join(library_plain_filename(name));
        copy_library(plain_path.as_std_path(), name, toolchain)
    }
}

pub fn copy_library(plain_path: &Path, lib_name: &str, toolchain: &str) -> Result<PathBuf> {
    assert_eq!(
        parse_plain_path(plain_path).as_deref(),
        Some(lib_name),
        "`plain_path` ({}) and `lib_name` ({lib_name}) do not correspond",
        plain_path.display(),
    );
    let filename_with_toolchain = library_filename_with_toolchain(lib_name, toolchain);
    let parent = plain_path
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory"))?;
    let path_with_toolchain = strip_deps(parent)
        .or_else(|| strip_build(parent))
        .unwrap_or_else(|| parent.to_path_buf())
        .join(filename_with_toolchain);
    // Avoid changing the copied library's modification time when Cargo reused the existing build
    // artifact. Rustc tracks loaded libraries as file dependencies, so an unnecessary overwrite
    // can cause Cargo to rebuild the crate being checked.
    if !files_are_equal(plain_path, &path_with_toolchain)? {
        copy(plain_path, &path_with_toolchain).with_context(|| {
            format!(
                "Could not copy `{}` to `{}`",
                plain_path.to_string_lossy(),
                path_with_toolchain.to_string_lossy()
            )
        })?;
    }

    Ok(path_with_toolchain)
}

fn strip_deps(path: &Path) -> Option<PathBuf> {
    let path = match (path.parent(), path.file_name()) {
        (Some(parent), Some(maybe_deps)) if maybe_deps == OsStr::new("deps") => parent,
        (_, _) => return None,
    };
    Some(path.to_path_buf())
}

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("[[:xdigit:]]{16}").unwrap());

#[allow(clippy::manual_let_else)]
fn strip_build(path: &Path) -> Option<PathBuf> {
    // <build-directory>/<profile>/build/<package-name>/<hash>/out
    let path = match (path.parent(), path.file_name()) {
        (Some(parent), Some(maybe_out)) if maybe_out == OsStr::new("out") => parent,
        (_, _) => return None,
    };
    let path = match (path.parent(), path.file_name()) {
        (Some(parent), Some(maybe_hash))
            if maybe_hash
                .to_str()
                .is_some_and(|filename| RE.is_match(filename)) =>
        {
            parent
        }
        (_, _) => return None,
    };
    let path = match (path.parent(), path.file_name()) {
        (Some(parent), Some(_maybe_package_name)) => parent,
        (_, _) => return None,
    };
    let path = match (path.parent(), path.file_name()) {
        (Some(parent), Some(maybe_build)) if maybe_build == OsStr::new("build") => parent,
        (_, _) => return None,
    };
    Some(path.to_path_buf())
}

fn files_are_equal(lhs: &Path, rhs: &Path) -> Result<bool> {
    const BUF_SIZE: usize = 16 * 1024;

    let metadata_lhs =
        metadata(lhs).with_context(|| format!("Could not get metadata for `{}`", lhs.display()))?;
    let metadata_rhs = match metadata(rhs) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Could not get metadata for `{}`", rhs.display()));
        }
    };

    if metadata_lhs.len() != metadata_rhs.len() {
        return Ok(false);
    }

    let mut file_lhs =
        File::open(lhs).with_context(|| format!("Could not open `{}`", lhs.display()))?;
    let mut file_rhs =
        File::open(rhs).with_context(|| format!("Could not open `{}`", rhs.display()))?;
    let mut buf_lhs = [0; BUF_SIZE];
    let mut buf_rhs = [0; BUF_SIZE];

    loop {
        let count = file_lhs
            .read(&mut buf_lhs)
            .with_context(|| format!("Could not read `{}`", lhs.display()))?;
        if count == 0 {
            let count = file_rhs
                .read(&mut buf_rhs[..1])
                .with_context(|| format!("Could not read `{}`", rhs.display()))?;
            return Ok(count == 0);
        }
        if let Err(error) = file_rhs.read_exact(&mut buf_rhs[..count]) {
            return if error.kind() == ErrorKind::UnexpectedEof {
                Ok(false)
            } else {
                Err(error).with_context(|| format!("Could not read `{}`", rhs.display()))
            };
        }
        if buf_lhs[..count] != buf_rhs[..count] {
            return Ok(false);
        }
    }
}
