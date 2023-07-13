//! This crate provides macros for creating [Dylint] libraries, and utilities for creating
//! configurable libraries.
//!
//! **Contents**
//!
//! - [`dylint_library!`]
//! - [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]
//! - [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]
//! - [Configurable libraries]
//!
//! # `dylint_library!`
//!
//! The `dylint_library!` macro expands to the following:
//!
//! ```rust,ignore
//! #[allow(unused_extern_crates)]
//! extern crate rustc_driver;
//!
//! #[no_mangle]
//! pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
//!     std::ffi::CString::new($crate::DYLINT_VERSION)
//!         .unwrap()
//!         .into_raw()
//! }
//! ```
//!
//! If your library uses the `dylint_library!` macro and the [`dylint-link`] tool, then all you
//! should have to do is implement the [`register_lints`] function. See the [examples] in this
//! repository.
//!
//! # `declare_late_lint!`, etc.
//!
//! If your library contains just one lint, using `declare_late_lint!`, etc. can make your code more
//! concise. Each of these macros requires the same arguments as [`declare_lint!`], and wraps the
//! following:
//!
//! - a call to `dylint_library!`
//! - an implementation of the `register_lints` function
//! - a call to `declare_lint!`
//! - a call to [`declare_lint_pass!`]
//!
//! For example, `declare_late_lint!(vis NAME, Level, "description")` expands to the following:
//!
//! ```rust,ignore
//! dylint_linting::dylint_library!();
//!
//! extern crate rustc_lint;
//! extern crate rustc_session;
//!
//! #[no_mangle]
//! pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
//!     dylint_linting::init_config(sess);
//!     lint_store.register_lints(&[NAME]);
//!     lint_store.register_late_pass(|_| Box::new(Name));
//! }
//!
//! rustc_session::declare_lint!(vis NAME, Level, "description");
//!
//! rustc_session::declare_lint_pass!(Name => [NAME]);
//! ```
//!
//! `declare_early_lint!` and `declare_pre_expansion_lint!` are defined similarly.
//!
//! # `impl_late_lint!`, etc.
//!
//! `impl_late_lint!`, etc. are like `declare_late_lint!`, etc. except:
//!
//! - each calls [`impl_lint_pass!`] instead of `declare_lint_pass!`;
//! - each requires an additional argument to specify the value of the lint's [`LintPass`]
//!   structure.
//!
//! That is, `impl_late_lint!`'s additional argument is what goes here:
//!
//! ```rust,ignore
//!     lint_store.register_late_pass(|_| Box::new(...));
//!                                                ^^^
//! ```
//!
//! An example use of `impl_pre_expansion_lint!` can be found in [`env_cargo_path`] in this
//! repository.
//!
//! # Configurable libraries
//!
//! Libraries can be configured by including a `dylint.toml` file in the target workspace's root
//! directory. This crate provides the following functions for reading and parsing `dylint.toml`
//! files:
//!
//! - [`config_or_default`]
//! - [`config`]
//! - [`config_toml`]
//! - [`init_config`]
//! - [`try_init_config`]
//!
//! A configurable library containing just one lint will typically have a `lib.rs` file of the
//! following form:
//!
//! ```rust,ignore
//! dylint_linting::impl_late_lint! {
//!     ...,
//!     LintName::new()
//! }
//!
//! // Lint configuration
//! #[derive(Default, serde::Deserialize)]
//! struct Config {
//!     boolean: bool,
//!     strings: Vec<String>,
//! }
//!
//! // Keep a copy of the configuration in the `LintPass` structure.
//! struct LintName {
//!     config: Config,
//! }
//!
//! // Read the configuration from the `dylint.toml` file, or use the default configuration if
//! // none is present.
//! impl LintName {
//!     pub fn new() -> Self {
//!         Self {
//!             config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
//!         }
//!     }
//! }
//! ```
//!
//! For a concrete example of a `lib.rs` file with this form, see the
//! [`non_local_effect_before_error_return`] library in this repository.
//!
//! A library containing more than one lint must implement the `register_lints` function without
//! relying on the above macros. If the library is configurable, then its `register_lints` function
//! should include a call to `dylint_linting::init_config`, as in the following example:
//!
//! ```rust,ignore
//! #[no_mangle]
//! pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
//!     // `init_config` or `try_init_config` must be called before `config_or_default`, `config`,
//!     // or `config_toml` is called.
//!     dylint_linting::init_config(sess);
//!
//!     lint_store.register_lints(&[FIRST_LINT_NAME, SECOND_LINT_NAME]);
//!
//!     lint_store.register_late_pass(|_| Box::new(LintPassName::new()));
//! }
//! ```
//!
//! Additional documentation on `config_or_default`, etc. can be found on [docs.rs].
//!
//! [`config_or_default`]: crate::config_or_default
//! [`config_toml`]: crate::config_toml
//! [`config`]: crate::config
//! [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]: #declare_late_lint-etc
//! [`declare_lint!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint.html
//! [`declare_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint_pass.html
//! [`dylint-link`]: ../../dylint-link
//! [`dylint_library!`]: #dylint_library
//! [`env_cargo_path`]: ../../examples/general/env_cargo_path/src/lib.rs
//! [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]: #impl_late_lint-etc
//! [`impl_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.impl_lint_pass.html
//! [`init_config`]: crate::init_config
//! [`lintpass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LintPass.html
//! [`non_local_effect_before_error_return`]: ../../examples/general/non_local_effect_before_error_return/src/lib.rs
//! [`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
//! [`try_init_config`]: crate::try_init_config
//! [configurable libraries]: #configurable-libraries
//! [docs.rs documentation]: https://docs.rs/dylint_linting/latest/dylint_linting/
//! [docs.rs]: https://docs.rs/dylint_linting/latest/dylint_linting/
//! [dylint]: ../..
//! [examples]: ../../examples

#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[allow(unused_extern_crates)]
extern crate rustc_driver;

extern crate rustc_session;
extern crate rustc_span;

use dylint_internal::env;
use rustc_span::Symbol;
use std::{any::type_name, fs::read_to_string, path::PathBuf, sync::Mutex};
use thiserror::Error;

pub const DYLINT_VERSION: &str = "0.1.0";

pub use paste;

// smoelius: Including `extern crate rustc_driver` causes the library to link against
// `librustc_driver.so`, which dylint-driver also links against. So, essentially, the library uses
// dylint-driver's copy of the Rust compiler crates.
#[macro_export]
macro_rules! dylint_library {
    () => {
        #[allow(unused_extern_crates)]
        extern crate rustc_driver;

        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
            std::ffi::CString::new($crate::DYLINT_VERSION)
                .unwrap()
                .into_raw()
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __declare_and_register_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $register_pass_method:ident, $pass:expr) => {
        $crate::dylint_library!();

        extern crate rustc_lint;
        extern crate rustc_session;

        #[allow(clippy::no_mangle_with_rust_abi)]
        #[no_mangle]
        pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
            $crate::init_config(sess);
            lint_store.register_lints(&[$NAME]);
            lint_store.$register_pass_method($pass);
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);
    };
}

#[rustversion::before(2022-09-08)]
#[doc(hidden)]
#[macro_export]
macro_rules! __make_late_closure {
    ($pass:expr) => {
        || Box::new($pass)
    };
}

// smoelius: Relevant PR and merge commit:
// - https://github.com/rust-lang/rust/pull/101501
// - https://github.com/rust-lang/rust/commit/87788097b776f8e3662f76627944230684b671bd
#[rustversion::since(2022-09-08)]
#[doc(hidden)]
#[macro_export]
macro_rules! __make_late_closure {
    ($pass:expr) => {
        |_| Box::new($pass)
    };
}

#[macro_export]
macro_rules! impl_pre_expansion_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_pre_expansion_pass,
            || Box::new($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! impl_early_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_early_pass,
            || Box::new($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! impl_late_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_late_pass,
            $crate::__make_late_closure!($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_pre_expansion_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_pre_expansion_pass,
                || Box::new([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_early_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_early_pass,
                || Box::new([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_late_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_late_pass,
                $crate::__make_late_closure!([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug)]
pub struct ConfigError {
    inner: ConfigErrorInner,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> From<T> for ConfigError
where
    ConfigErrorInner: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            inner: ConfigErrorInner::from(value),
        }
    }
}

#[derive(Debug, Error)]
enum ConfigErrorInner {
    #[error("cargo metadata error: {0}")]
    CargoMetadata(#[from] cargo_metadata::Error),
    #[error("io error: {0}: {1}")]
    Io(String, std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Other(String),
}

static CONFIG_TABLE: Mutex<Option<toml::value::Table>> = Mutex::new(None);

/// Reads and deserializes an entry from the workspace's `dylint.toml` file, and returns the default
/// value if the entry is not present.
///
/// - If the target workspace's `dylint.toml` file contains key `name` and its value can be
///   deserializes as `T`, `config_or_default` returns the deserialized value.
/// - If the target workspace's `dylint.toml` file does not exist or does not contain key `name`,
///   `config_or_default` returns `T::default()`.
/// - If an error occurs (e.g., the value cannot be deserialized as `T`), `config_or_default`
///   panics.
///
/// Note: `init_config` or `try_init_config` must be called before `config_or_default` is called.
/// However, the `register_lints` function generated by `impl_late_lint`, etc. includes a call to
/// `init_config`.
pub fn config_or_default<T: Default + serde::de::DeserializeOwned>(name: &str) -> T {
    config::<T>(name).map_or_else(
        |error| {
            panic!(
                "Could not parse config as `{}`: {}",
                type_name::<T>(),
                error
            )
        },
        Option::unwrap_or_default,
    )
}

/// Reads and deserializes an entry from the workspace's `dylint.toml` file.
///
/// Returns:
/// - `Ok(Some(...))` if the target workspace's `dylint.toml` file contains key `name` and its value
///   can be deserialized as `T`
/// - `Ok(None)` if the target workspace's `dylint.toml` file does not exist or does not contain key
///   `name`
/// - `Err(...)` if an error occurs (e.g., the value cannot be deserialized as `T`)
///
/// Note: `init_config` or `try_init_config` must be called before `config` is called. However, the
/// `register_lints` function generated by `impl_late_lint`, etc. includes a call to `init_config`.
pub fn config<T: serde::de::DeserializeOwned>(name: &str) -> ConfigResult<Option<T>> {
    let toml = config_toml(name)?;
    toml.map(toml::Value::try_into::<T>)
        .transpose()
        .map_err(Into::into)
}

/// Reads an entry from the workspace's `dylint.toml` file as a raw `toml::Value`.
///
/// Returns:
/// - `Ok(Some(...))` if the target workspace's `dylint.toml` file contains key `name`
/// - `Ok(None)` if the target workspace's `dylint.toml` file does not exist or does not contain key
///   `name`
/// - `Err(...)` if an error occurs (e.g., `init_config` was not called)
///
/// Note: `init_config` or `try_init_config` must be called before `config_toml` is called. However,
/// the `register_lints` function generated by `impl_late_lint`, etc. includes a call to
/// `init_config`.
pub fn config_toml(name: &str) -> ConfigResult<Option<toml::Value>> {
    let config_table = CONFIG_TABLE.lock().unwrap();
    let config_table = config_table.as_ref().ok_or_else(|| {
        ConfigErrorInner::Other(
            "Config is not initialized; `init_config` should have been called from `register_lints`"
                .into(),
        )
    })?;
    Ok(config_table.get(name).cloned())
}

/// A wrapper around `try_init_config`. Calls `rustc_session::early_error` if `try_init_config`
/// returns an error.
///
/// Note: `init_config` or `try_init_config` must be called before `config_or_default`, `config`, or
/// `config_toml` is called. However, the `register_lints` function generated by `impl_late_lint`,
/// etc. includes a call to `init_config`.
pub fn init_config(sess: &rustc_session::Session) {
    try_init_config(sess).unwrap_or_else(|err| {
        let msg = format!("could not read configuration file: {err}");
        early_error(msg);
    });
}

/// Reads the target workspace's `dylint.toml` file and parses it as a `toml::value::Table`.
///
/// Note: `init_config` or `try_init_config` must be called before `config_or_default`, `config`, or
/// `config_toml` is called. However, the `register_lints` function generated by `impl_late_lint`,
/// etc. includes a call to `init_config`.
pub fn try_init_config(sess: &rustc_session::Session) -> ConfigResult<()> {
    let mut config_table = CONFIG_TABLE.lock().unwrap();

    if config_table.is_some() {
        return Ok(());
    }

    let value = if let Ok(value) = std::env::var(env::DYLINT_TOML) {
        sess.parse_sess.env_depinfo.lock().insert((
            Symbol::intern(env::DYLINT_TOML),
            Some(Symbol::intern(&value)),
        ));
        Some(value)
    } else if let Some(local_crate_source_file) = local_crate_source_file(sess).and_then(|path| {
        if path == PathBuf::new() {
            None
        } else {
            Some(path)
        }
    }) {
        let local_crate_source_file = local_crate_source_file.canonicalize().map_err(|error| {
            ConfigErrorInner::Io(
                format!("Could not canonicalize {local_crate_source_file:?}"),
                error,
            )
        })?;

        let parent = local_crate_source_file
            .parent()
            .ok_or_else(|| ConfigErrorInner::Other("Could not get parent directory".into()))?;

        let result = cargo_metadata::MetadataCommand::new()
            .current_dir(parent)
            .no_deps()
            .exec();

        match result {
            Err(cargo_metadata::Error::CargoMetadata { stderr })
                if stderr.contains("could not find `Cargo.toml`") =>
            {
                None
            }
            _ => {
                let cargo_metadata::Metadata { workspace_root, .. } = result?;

                let dylint_toml = workspace_root.join("dylint.toml");

                if dylint_toml.try_exists().map_err(|error| {
                    ConfigErrorInner::Io(format!("`try_exists` failed for {dylint_toml:?}"), error)
                })? {
                    let value = read_to_string(&dylint_toml).map_err(|error| {
                        ConfigErrorInner::Io(
                            format!("`read_to_string` failed for {dylint_toml:?}"),
                            error,
                        )
                    })?;
                    sess.parse_sess
                        .file_depinfo
                        .lock()
                        .insert(Symbol::intern(dylint_toml.as_str()));
                    Some(value)
                } else {
                    None
                }
            }
        }
    } else {
        None
    };

    let toml: Option<toml::Value> = value.as_deref().map(toml::from_str).transpose()?;

    let table = toml
        .map(|toml| {
            toml.as_table()
                .cloned()
                .ok_or_else(|| ConfigErrorInner::Other("Value is not a table".into()))
        })
        .transpose()?;

    config_table.replace(table.unwrap_or_default());

    Ok(())
}

#[rustversion::before(2023-01-19)]
fn local_crate_source_file(sess: &rustc_session::Session) -> Option<PathBuf> {
    sess.local_crate_source_file.clone()
}

// smoelius: Relevant PR and merge commit:
// - https://github.com/rust-lang/rust/pull/106810
// - https://github.com/rust-lang/rust/commit/65d2f2a5f9c323c88d1068e8e90d0b47a20d491c
#[rustversion::since(2023-01-19)]
fn local_crate_source_file(sess: &rustc_session::Session) -> Option<PathBuf> {
    sess.local_crate_source_file()
}

#[rustversion::before(2023-06-28)]
fn early_error(msg: String) -> ! {
    rustc_session::early_error(
        rustc_session::config::ErrorOutputType::default(),
        Box::leak(msg.into_boxed_str()) as &str,
    )
}

#[rustversion::since(2023-06-28)]
extern crate rustc_errors;

#[rustversion::since(2023-06-28)]
fn early_error(msg: impl Into<rustc_errors::DiagnosticMessage>) -> ! {
    let handler =
        rustc_session::EarlyErrorHandler::new(rustc_session::config::ErrorOutputType::default());
    handler.early_error(msg)
}
