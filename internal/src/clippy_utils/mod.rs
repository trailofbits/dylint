use anyhow::{Context, Result, anyhow, bail};
use semver::Version;
use std::{
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item, Value};

mod repository;
pub use repository::{clippy_repository, parse_as_nightly};

mod revs_no_preinstall;
pub use revs_no_preinstall::{Rev, Revs};

#[allow(clippy::module_name_repetitions)]
pub fn clippy_utils_version_from_rust_version(rust_version: &str) -> Result<String> {
    Version::parse(rust_version.strip_prefix("rust-").unwrap_or(rust_version))
        .map(|version| Version::new(0, version.major, version.minor).to_string())
        .map_err(Into::into)
}

#[allow(clippy::module_name_repetitions)]
pub fn clippy_utils_package_version(path: &Path) -> Result<String> {
    let cargo_toml = path.join("clippy_utils/Cargo.toml");
    let contents = read_to_string(&cargo_toml).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            cargo_toml.to_string_lossy(),
        )
    })?;
    let table = toml::from_str::<toml::Table>(&contents)?;
    table
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("version"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))
}

pub fn set_clippy_utils_dependency_revision(path: &Path, rev: &str) -> Result<()> {
    let cargo_toml = path.join("Cargo.toml");
    let contents = read_to_string(&cargo_toml).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            cargo_toml.to_string_lossy(),
        )
    })?;
    let mut document = contents.parse::<DocumentMut>()?;
    // smoelius: First check `dependencies` for `clippy_utils`.
    let mut clippy_utils = document
        .as_table_mut()
        .get_mut("dependencies")
        .and_then(Item::as_table_mut)
        .and_then(|table| table.get_mut("clippy_utils"));
    // smoelius: It it's not found there, check `workspace.dependencies`.
    if clippy_utils.is_none() {
        clippy_utils = document
            .as_table_mut()
            .get_mut("workspace")
            .and_then(Item::as_table_mut)
            .and_then(|table| table.get_mut("dependencies"))
            .and_then(Item::as_table_mut)
            .and_then(|table| table.get_mut("clippy_utils"));
    }
    clippy_utils
        .and_then(Item::as_inline_table_mut)
        .and_then(|table| table.get_mut("rev"))
        .map(|value| *value = Value::from(rev))
        .ok_or_else(|| anyhow!("Could not set `clippy_utils` revision"))?;
    write(cargo_toml, document.to_string()).map_err(Into::into)
}

/// Extracts the `toolchain.channel` setting from a `rust-toolchain` or `rust-toolchain.toml` file
pub fn toolchain_channel(path: &Path) -> Result<String> {
    let Some((_, contents)) = read_rust_toolchain_file(path)? else {
        bail!("Could not find rust-toolchain file at `{}`", path.display());
    };
    let table = toml::from_str::<toml::Table>(&contents)?;
    table
        .get("toolchain")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("channel"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine Rust toolchain channel"))
}

/// Sets `toolchain.channel` in a `rust-toolchain` or `rust-toolchain.toml` file
pub fn set_toolchain_channel(path: &Path, channel: &str) -> Result<()> {
    let Some((path_used, contents)) = read_rust_toolchain_file(path)? else {
        bail!("Could not find rust-toolchain file at `{}`", path.display());
    };
    let mut document = contents.parse::<DocumentMut>()?;
    document
        .as_table_mut()
        .get_mut("toolchain")
        .and_then(Item::as_table_mut)
        .and_then(|table| table.get_mut("channel"))
        .and_then(Item::as_value_mut)
        .map(|value| *value = Value::from(channel))
        .ok_or_else(|| anyhow!("Could not set Rust toolchain channel"))?;
    write(path_used, document.to_string()).map_err(Into::into)
}

pub fn read_rust_toolchain_file(path: &Path) -> Result<Option<(PathBuf, String)>> {
    // smoelius: Rustup gives precedence to `rust-toolchain` over `rust-toolchain.toml`:
    // https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file
    let rust_toolchain_path = path.join("rust-toolchain");
    let rust_toolchain_toml_path = path.join("rust-toolchain.toml");
    if rust_toolchain_path.try_exists().with_context(|| {
        format!(
            "Could not determine whether `{}` exists",
            rust_toolchain_path.display(),
        )
    })? {
        let contents = read_to_string(&rust_toolchain_path).with_context(|| {
            format!(
                "`read_to_string` failed for `{}`",
                rust_toolchain_path.display(),
            )
        })?;
        return Ok(Some((rust_toolchain_path, contents)));
    }
    if rust_toolchain_toml_path.try_exists().with_context(|| {
        format!(
            "Could not determine whether `{}` exists",
            rust_toolchain_toml_path.display(),
        )
    })? {
        let contents = read_to_string(&rust_toolchain_toml_path).with_context(|| {
            format!(
                "`read_to_string` failed for `{}`",
                rust_toolchain_toml_path.display(),
            )
        })?;
        return Ok(Some((rust_toolchain_toml_path, contents)));
    }
    Ok(None)
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_dir;
    use tempfile::tempdir;

    const BEFORE: &str = r#"[toolchain]
channel = "nightly-2025-01-02"
components = ["llvm-tools-preview", "rustc-dev"]
"#;

    const AFTER: &str = r#"[toolchain]
channel = "nightly-2025-03-04"
components = ["llvm-tools-preview", "rustc-dev"]
"#;

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn set_toolchain_channel_rust_toolchain() {
        check_set_toolchain_channel("rust-toolchain");
    }

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn set_toolchain_channel_rust_toolchain_toml() {
        check_set_toolchain_channel("rust-toolchain.toml");
    }

    fn check_set_toolchain_channel(filename: &str) {
        let tempdir = tempdir().unwrap();
        write(tempdir.path().join(filename), BEFORE).unwrap();

        set_toolchain_channel(tempdir.path(), "nightly-2025-03-04").unwrap();

        assert_eq!(
            "nightly-2025-03-04",
            toolchain_channel(tempdir.path()).unwrap()
        );

        // The channel should be the only thing that changed.
        assert_eq!(
            AFTER,
            read_to_string(tempdir.path().join(filename)).unwrap()
        );

        // The file that was present should have been written; the other should not have been
        // created.
        assert_eq!(1, read_dir(&tempdir).unwrap().count());
    }
}
