use crate::{rustup::SanitizeEnvironment, CommandExt};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn build() -> Result<()> {
    // smoelius: The examples use `dylint-link` as the linker, so it must be built first.
    #[allow(unknown_lints, abs_home_path)]
    crate::cargo::build("dylint-link")
        .build()
        .sanitize_environment()
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../dylint-link"))
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

pub fn iter(workspace: bool) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    #[allow(unknown_lints, abs_home_path)]
    let path_buf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    // smoelius: Use `cargo_util::paths::normalize_path` instead of `canonicalize` so as not to
    // "taint" the path with a path prefix on Windows.
    let examples = cargo_util::paths::normalize_path(&path_buf);
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
                    format!("Could not determine whether {rust_toolchain_path:?} exists")
                })?
            {
                return Ok(Some(path.to_path_buf()));
            }
            if !workspace
                && cargo_toml_path.try_exists().with_context(|| {
                    format!("Could not determine whether {cargo_toml_path:?} exists")
                })?
            {
                return Ok(Some(path.to_path_buf()));
            }
            Ok(None)
        })
        .filter_map(Result::transpose))
}
