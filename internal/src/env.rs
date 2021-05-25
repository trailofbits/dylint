use anyhow::{anyhow, Result};

pub const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
pub const CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
pub const CLIPPY_DISABLE_DOCS_LINKS: &str = "CLIPPY_DISABLE_DOCS_LINKS";
pub const CLIPPY_DRIVER_PATH: &str = "CLIPPY_DRIVER_PATH";
pub const DYLINT_DRIVER_PATH: &str = "DYLINT_DRIVER_PATH";
pub const DYLINT_LIBRARY_PATH: &str = "DYLINT_LIBRARY_PATH";
pub const DYLINT_LIBS: &str = "DYLINT_LIBS";
pub const DYLINT_LIST: &str = "DYLINT_LIST";
pub const DYLINT_RUSTFLAGS: &str = "DYLINT_RUSTFLAGS";
pub const HOME: &str = "HOME";
pub const RUSTC_WORKSPACE_WRAPPER: &str = "RUSTC_WORKSPACE_WRAPPER";
pub const RUSTFLAGS: &str = "RUSTFLAGS";
pub const RUSTUP_HOME: &str = "RUSTUP_HOME";
pub const RUSTUP_TOOLCHAIN: &str = "RUSTUP_TOOLCHAIN";

pub fn var(key: &str) -> Result<String> {
    std::env::var(key).map_err(|err| anyhow!(format!("{}: {}", err, key)))
}
