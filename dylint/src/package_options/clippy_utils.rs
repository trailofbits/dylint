use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use semver::Version;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};
use toml_edit::{Document, Item};

lazy_static! {
    pub static ref CLIPPY_UTILS_CARGO_TOML: PathBuf = Path::new("clippy_utils").join("Cargo.toml");
}

#[allow(clippy::module_name_repetitions)]
pub fn clippy_utils_version_from_rust_version(rust_version: &str) -> Result<String> {
    Version::parse(rust_version.strip_prefix("rust-").unwrap_or(rust_version))
        .map(|version| Version::new(0, version.major, version.minor).to_string())
        .map_err(Into::into)
}

pub fn version(path: &Path) -> Result<String> {
    let cargo_toml = path.join(&*CLIPPY_UTILS_CARGO_TOML);
    let file = read_to_string(&cargo_toml).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            cargo_toml.to_string_lossy(),
        )
    })?;
    let document = file.parse::<Document>()?;
    document
        .as_table()
        .get("package")
        .and_then(Item::as_table)
        .and_then(|table| table.get("version"))
        .and_then(Item::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))
}

pub fn channel(path: &Path) -> Result<String> {
    let rust_toolchain = path.join("rust-toolchain");
    let file = read_to_string(&rust_toolchain).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            rust_toolchain.to_string_lossy(),
        )
    })?;
    file.lines()
        .find_map(|line| line.strip_prefix(r#"channel = ""#))
        .and_then(|line| line.strip_suffix('"'))
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine Rust toolchain channel"))
}
