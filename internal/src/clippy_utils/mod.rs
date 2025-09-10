use anyhow::{Context, Result, anyhow, bail};
use semver::Version;
use std::{
    fs::{read_to_string, write},
    path::Path,
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
    let rust_toolchain = path.join("rust-toolchain");
    let rust_toolchain_toml = path.join("rust-toolchain.toml");
    let contents = match read_to_string(&rust_toolchain) {
        Ok(contents) => contents,
        Err(error) => match read_to_string(&rust_toolchain_toml) {
            Ok(contents) => contents,
            Err(error_toml) => {
                bail!(
                    "`read_to_string` failed for both `{}` and `{}`: {:#?}",
                    rust_toolchain.to_string_lossy(),
                    rust_toolchain_toml.to_string_lossy(),
                    [error, error_toml]
                );
            }
        },
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
    let rust_toolchain = path.join("rust-toolchain");
    let contents = read_to_string(&rust_toolchain).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            rust_toolchain.to_string_lossy(),
        )
    })?;
    let mut document = contents.parse::<DocumentMut>()?;
    document
        .as_table_mut()
        .get_mut("toolchain")
        .and_then(Item::as_table_mut)
        .and_then(|table| table.get_mut("channel"))
        .and_then(Item::as_value_mut)
        .map(|value| *value = Value::from(channel))
        .ok_or_else(|| anyhow!("Could not set Rust toolchain channel"))?;
    write(rust_toolchain, document.to_string()).map_err(Into::into)
}
