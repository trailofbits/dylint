use anyhow::Result;
use dylint_env as env;
use std::path::Path;
use std::{env::remove_var, fs::read_dir, path::PathBuf};

pub fn build() -> Result<()> {
    sanitize_environment();

    // smoelius: The examples use `dylint-link` as the linker, so it must be built first.
    dylint_testing::build(Some(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("dylint-link"),
    ))?;

    for example in iter()? {
        let example = example?;
        dylint_testing::build(Some(&example))?;
    }

    Ok(())
}

fn sanitize_environment() {
    remove_var(env::RUSTUP_TOOLCHAIN);
}

pub fn iter() -> Result<impl Iterator<Item = Result<PathBuf>>> {
    let iter = read_dir(env!("CARGO_MANIFEST_DIR"))?;
    Ok(iter
        .map(|entry| -> Result<Option<PathBuf>> {
            let entry = entry?;
            let path = entry.path();
            Ok(
                if path.is_dir() && path.file_name() != Some(std::ffi::OsStr::new("src")) {
                    Some(path)
                } else {
                    None
                },
            )
        })
        .filter_map(Result::transpose))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn examples() {
        sanitize_environment();

        for path in iter().unwrap() {
            let path = path.unwrap();
            assert!(std::process::Command::new("cargo")
                .current_dir(path)
                .args(&["test"])
                .status()
                .unwrap()
                .success());
        }
    }
}
