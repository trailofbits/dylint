use crate::rustup::SanitizeEnvironment;
use anyhow::{anyhow, Context, Result};
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

#[allow(unknown_lints)]
#[allow(env_cargo_path)]
pub fn build() -> Result<()> {
    // smoelius: The examples use `dylint-link` as the linker, so it must be built first.
    crate::cargo::build("dylint-link", false)
        .sanitize_environment()
        .current_dir(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("dylint-link"),
        )
        .success()?;

    for example in iter()? {
        let example = example?;
        let file_name = example
            .file_name()
            .ok_or_else(|| anyhow!("Could not get file name"))?;
        crate::cargo::build(&format!("example `{}`", file_name.to_string_lossy()), false)
            .sanitize_environment()
            .current_dir(&example)
            .success()?;
    }

    Ok(())
}

#[allow(unknown_lints)]
#[allow(env_cargo_path)]
pub fn iter() -> Result<impl Iterator<Item = Result<PathBuf>>> {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples");
    let iter = read_dir(&examples)
        .with_context(|| format!("`read_dir` failed for `{}`", examples.to_string_lossy()))?;
    Ok(iter
        .map(move |entry| -> Result<Option<PathBuf>> {
            let entry = entry.with_context(|| {
                format!("`read_dir` failed for `{}`", examples.to_string_lossy())
            })?;
            let path = entry.path();
            Ok(if path.is_dir() { Some(path) } else { None })
        })
        .filter_map(Result::transpose))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn examples() {
        for path in iter().unwrap() {
            let path = path.unwrap();
            let file_name = path.file_name().unwrap();
            crate::cargo::test(&format!("example `{}`", file_name.to_string_lossy()), false)
                .sanitize_environment()
                .current_dir(path)
                .success()
                .unwrap();
        }
    }
}
