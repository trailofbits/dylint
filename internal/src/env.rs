use anyhow::{Result, anyhow};

macro_rules! declare_const {
    ($var: ident) => {
        pub const $var: &str = stringify!($var);
    };
}

declare_const!(CARGO);
declare_const!(CARGO_CRATE_NAME);
declare_const!(CARGO_HOME);
declare_const!(CARGO_INCREMENTAL);
declare_const!(CARGO_MANIFEST_DIR);
declare_const!(CARGO_PKG_NAME);
declare_const!(CARGO_PRIMARY_PACKAGE);
declare_const!(CARGO_TARGET_DIR);
declare_const!(CARGO_TERM_COLOR);
declare_const!(CI);
declare_const!(CLIPPY_DISABLE_DOCS_LINKS);
declare_const!(CLIPPY_DRIVER_PATH);
declare_const!(DOCS_RS);
declare_const!(DYLINT_DRIVER_PATH);
declare_const!(DYLINT_LIBRARY_PATH);
declare_const!(DYLINT_LIBS);
declare_const!(DYLINT_LIST);
declare_const!(DYLINT_METADATA);
declare_const!(DYLINT_NO_DEPS);
declare_const!(DYLINT_RUSTFLAGS);
declare_const!(DYLINT_TOML);
declare_const!(GITHUB_TOKEN);
declare_const!(OUT_DIR);
declare_const!(PATH);
declare_const!(RUSTC);
declare_const!(RUSTC_WORKSPACE_WRAPPER);
declare_const!(RUSTFLAGS);
declare_const!(RUSTUP_HOME);
declare_const!(RUSTUP_TOOLCHAIN);
declare_const!(RUST_LOG);
declare_const!(TARGET);

/// Returns true if the environment variable `key` is set to a non-zero value.
///
/// # Examples
///
/// ```
/// use dylint_internal::env::enabled;
/// use std::env;
///
/// unsafe {
///     env::set_var("FOO", "1");
/// }
/// assert_eq!(enabled("FOO"), true);
///
/// unsafe {
///     env::set_var("FOO", "0");
/// }
/// assert_eq!(enabled("FOO"), false);
///
/// unsafe {
///     env::remove_var("FOO");
/// }
/// assert_eq!(enabled("FOO"), false);
/// ```
#[must_use]
pub fn enabled(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| value != "0")
}

/// A wrapper around `std::env::var` that converts the error into an `anyhow::Error`.
pub fn var(key: &str) -> Result<String> {
    std::env::var(key).map_err(|err| anyhow!(format!("{err}: {key}")))
}
