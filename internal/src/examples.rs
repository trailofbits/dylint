use crate::{CommandExt, rustup::SanitizeEnvironment};
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn build() -> Result<()> {
    // smoelius: The examples use `dylint-link` as the linker, so it must be built first.
    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    crate::cargo::build("dylint-link")
        .build()
        .sanitize_environment()
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../dylint-link"))
        .success()?;

    for example in iter(true)? {
        let example = example?;
        let file_name = example
            .file_name()
            .ok_or_else(|| anyhow!("Could not get file name"))?;
        crate::cargo::build(&format!("example `{}`", file_name.to_string_lossy()))
            .build()
            .sanitize_environment()
            .current_dir(&example)
            .success()?;
    }

    Ok(())
}

/// Returns an iterator over the example libraries' directories.
///
/// - If the `workspace` argument is true, workspace directories (e.g., general and supplementary)
///   are included, but their member directories are not.
/// - If the `workspace` argument is false, the member directories are included, but the workspace
///   directories are not.
pub fn iter(workspace: bool) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples"));
    // smoelius: Use `cargo_util::paths::normalize_path` instead of `canonicalize` so as not to
    // "taint" the path with a path prefix on Windows.
    let examples = cargo_util::paths::normalize_path(path);
    let iter = WalkDir::new(examples)
        .into_iter()
        .filter_entry(|entry| entry.depth() <= 2);
    Ok(iter
        .map(move |entry| -> Result<Option<PathBuf>> {
            let entry = entry?;
            let path = entry.path();
            let rust_toolchain_path = path.join("rust-toolchain");
            let cargo_toml_path = path.join("Cargo.toml");
            if entry.depth() < 1 || !path.is_dir() {
                return Ok(None);
            }
            if workspace
                && rust_toolchain_path.try_exists().with_context(|| {
                    format!(
                        "Could not determine whether `{}` exists",
                        rust_toolchain_path.display()
                    )
                })?
            {
                return Ok(Some(path.to_path_buf()));
            }
            if !workspace
                && cargo_toml_path.try_exists().with_context(|| {
                    format!(
                        "Could not determine whether `{}` exists",
                        cargo_toml_path.display()
                    )
                })?
            {
                return Ok(Some(path.to_path_buf()));
            }
            Ok(None)
        })
        .filter_map(Result::transpose))
}
