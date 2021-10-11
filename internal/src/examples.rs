use crate::rustup::SanitizeEnvironment;
use anyhow::{Context, Result};
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

pub fn build() -> Result<()> {
    // smoelius: The examples use `dylint-link` as the linker, so it must be built first.
    crate::build()
        .sanitize_environment()
        .current_dir(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("dylint-link"),
        )
        .success()?;

    for example in iter()? {
        let example = example?;
        crate::build()
            .sanitize_environment()
            .current_dir(&example)
            .success()?;
    }

    Ok(())
}

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
            crate::test()
                .sanitize_environment()
                .current_dir(path)
                .success()
                .unwrap();
        }
    }
}
