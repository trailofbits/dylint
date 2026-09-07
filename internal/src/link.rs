use crate::{library_filename_with_toolchain, parse_plain_path};
use anyhow::{Context, Result, anyhow};
use std::{
    ffi::OsStr,
    fs::{File, copy, metadata},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
};

pub fn copy_library(plain_path: &Path, lib_name: &str, toolchain: &str) -> Result<()> {
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
    let path_with_toolchain = strip_deps(parent).join(filename_with_toolchain);
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

    Ok(())
}

fn strip_deps(path: &Path) -> PathBuf {
    if path.file_name() == Some(OsStr::new("deps")) {
        path.parent()
    } else {
        None
    }
    .unwrap_or(path)
    .to_path_buf()
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
