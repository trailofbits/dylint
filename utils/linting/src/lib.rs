//! This crate provides macros for creating [Dylint] libraries, and utilities for creating
//! configurable libraries.
//!
//! **Contents**
//!
//! - [`dylint_library!`]
//! - [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]
//! - [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]
//! - [`constituent` feature]
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
//! #[unsafe(no_mangle)]
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
//! #[unsafe(no_mangle)]
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
//! # `constituent` feature
//!
//! Enabling the package-level `constituent` feature changes the way the above macros work.
//! Specifically, it causes them to _exclude_:
//!
//! - the call to `dylint_library!`
//! - the use of `#[unsafe(no_mangle)]` just prior to the declaration of `register_lints`
//!
//! Such changes facilitate inclusion of a lint declared with one of the above macros into a larger
//! library. That is:
//!
//! - With the feature turned off, the lint can be built as a library by itself.
//! - With the feature turned on, the lint can be built as part of a larger library, alongside other
//!   lints.
//!
//! The [general-purpose] and [supplementary] lints in this repository employ this technique.
//! That is, each general-purpose lint can be built as a library by itself, or as part of the
//! [`general` library]. An analogous statement applies to the supplementary lints and the
//! [`supplementary` library]. The `constituent` feature is the underlying mechanism that makes this
//! work.
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
//! #[unsafe(no_mangle)]
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
//! [Configurable libraries]: #configurable-libraries
//! [Dylint]: https://github.com/trailofbits/dylint/tree/master
//! [`LintPass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LintPass.html
//! [`config_or_default`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config_or_default.html
//! [`config_toml`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config_toml.html
//! [`config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config.html
//! [`constituent` feature]: #constituent-feature
//! [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]: #declare_late_lint-etc
//! [`declare_lint!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint.html
//! [`declare_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint_pass.html
//! [`dylint-link`]: https://github.com/trailofbits/dylint/tree/master/dylint-link
//! [`dylint_library!`]: #dylint_library
//! [`general` library]: https://github.com/trailofbits/dylint/tree/master/examples/general/src/lib.rs
//! [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]: #impl_late_lint-etc
//! [`impl_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.impl_lint_pass.html
//! [`init_config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.init_config.html
//! [`non_local_effect_before_error_return`]: https://github.com/trailofbits/dylint/tree/master/examples/general/non_local_effect_before_error_return/src/lib.rs
//! [`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
//! [`supplementary` library]: https://github.com/trailofbits/dylint/tree/master/examples/supplementary/src/lib.rs
//! [`try_init_config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.try_init_config.html
//! [docs.rs documentation]: https://docs.rs/dylint_linting/latest/dylint_linting/
//! [docs.rs]: https://docs.rs/dylint_linting/latest/dylint_linting/
//! [examples]: https://github.com/trailofbits/dylint/tree/master/examples
//! [general-purpose]: https://github.com/trailofbits/dylint/tree/master/examples/general
//! [supplementary]: https://github.com/trailofbits/dylint/tree/master/examples/supplementary

#![feature(rustc_private)]
#![allow(clippy::useless_attribute)]
#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![warn(unused_extern_crates)]

#[allow(unused_extern_crates)]
extern crate rustc_driver;

extern crate rustc_session;
extern crate rustc_span;

use dylint_internal::{config, env};
use rustc_span::Symbol;
use std::{
    any::type_name,
    path::{Path, PathBuf},
};

pub use config::{Error as ConfigError, Result as ConfigResult};

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
        #[unsafe(no_mangle)]
        pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
            std::ffi::CString::new($crate::DYLINT_VERSION)
                .unwrap()
                .into_raw()
        }
    };
}

#[cfg(not(feature = "constituent"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __maybe_exclude {
    ($item:item) => {
        $item
    };
}

#[cfg(feature = "constituent")]
#[doc(hidden)]
#[macro_export]
macro_rules! __maybe_exclude {
    ($item:item) => {};
}

#[cfg(not(feature = "constituent"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __maybe_mangle {
    ($item:item) => {
        #[unsafe(no_mangle)]
        $item
    };
}

#[cfg(feature = "constituent")]
#[doc(hidden)]
#[macro_export]
macro_rules! __maybe_mangle {
    ($item:item) => {
        $item
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __declare_and_register_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $register_pass_method:ident, $pass:expr) => {
        $crate::__maybe_exclude! {
            $crate::dylint_library!();
        }

        extern crate rustc_lint;
        extern crate rustc_session;

        $crate::__maybe_mangle! {
            #[allow(clippy::no_mangle_with_rust_abi)]
            pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
                $crate::init_config(sess);
                lint_store.register_lints(&[$NAME]);
                lint_store.$register_pass_method($pass);
            }
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
///
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
///
/// - `Ok(Some(...))` if the target workspace's `dylint.toml` file contains key `name`
/// - `Ok(None)` if the target workspace's `dylint.toml` file does not exist or does not contain key
///   `name`
/// - `Err(...)` if an error occurs (e.g., `init_config` was not called)
///
/// Note: `init_config` or `try_init_config` must be called before `config_toml` is called. However,
/// the `register_lints` function generated by `impl_late_lint`, etc. includes a call to
/// `init_config`.
pub fn config_toml(name: &str) -> ConfigResult<Option<toml::Value>> {
    let Some(config_table) = config::get() else {
        return Err(ConfigError::other(
            "Config is not initialized; `init_config` should have been called from \
             `register_lints`"
                .into(),
        ));
    };
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

trait ParseSess {
    fn parse_sess(&self) -> &rustc_session::parse::ParseSess;
}

impl ParseSess for rustc_session::Session {
    #[rustversion::before(2024-03-05)]
    fn parse_sess(&self) -> &rustc_session::parse::ParseSess {
        &self.parse_sess
    }

    #[rustversion::since(2024-03-05)]
    fn parse_sess(&self) -> &rustc_session::parse::ParseSess {
        &self.psess
    }
}

/// Reads the target workspace's `dylint.toml` file and parses it as a `toml::value::Table`.
///
/// Note: `init_config` or `try_init_config` must be called before `config_or_default`, `config`, or
/// `config_toml` is called. However, the `register_lints` function generated by `impl_late_lint`,
/// etc. includes a call to `init_config`.
pub fn try_init_config(sess: &rustc_session::Session) -> ConfigResult<()> {
    let result = try_init_config_guarded(sess);

    // smoelius: If we're returning `Ok(())`, ensure that `config::get()` will later return
    // `Some(..)`.
    if result.is_ok() && config::get().is_none() {
        config::init_from_string("").unwrap();
    }

    result
}

#[allow(clippy::empty_line_after_outer_attr)]
#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
fn try_init_config_guarded(sess: &rustc_session::Session) -> ConfigResult<()> {
    if config::get().is_some() {
        return Ok(());
    }

    if let Ok(value) = std::env::var(env::DYLINT_TOML) {
        config::init_from_string(&value)?;
        sess.parse_sess().env_depinfo.lock().insert((
            Symbol::intern(env::DYLINT_TOML),
            Some(Symbol::intern(&value)),
        ));
        return Ok(());
    }

    let Some(local_crate_source_file) =
        local_crate_source_file(sess).filter(|path| *path != PathBuf::new())
    else {
        return Ok(());
    };

    #[rustfmt::skip]
    // smoelius: Canonicalizing `local_crate_source_file` causes errors like the following on
    // Windows:
    //
    //   error: could not read configuration file: cargo metadata error: `cargo metadata` exited with an error: error: failed to load manifest for dependency `await_holding_span_guard`
    //
    //          Caused by:
    //            failed to parse manifest at `D:\a\dylint\dylint\examples\general\await_holding_span_guard\Cargo.toml`
    //
    //          Caused by:
    //            error inheriting `clippy_utils` from workspace root manifest's `workspace.dependencies.clippy_utils`
    //
    //          Caused by:
    //            `workspace.dependencies` was not defined
    //
    // The issue is that `canonicalize` prepends `\\?\` to the path, and such "verbatim" paths
    // cause problems for Cargo. See the following GitHub issue for more information:
    // https://github.com/rust-lang/cargo/issues/9770#issuecomment-993069234
    //
    // For reasons that I don't understand, fixing this problem in Cargo would be difficult.

    /* let local_crate_source_file = local_crate_source_file.canonicalize().map_err(|error| {
        ConfigErrorInner::Io(
            format!("Could not canonicalize {local_crate_source_file:?}"),
            error,
        )
    })?; */

    let mut parent = local_crate_source_file
        .parent()
        .ok_or_else(|| ConfigError::other("Could not get parent directory".into()))?;

    // smoelius: https://users.rust-lang.org/t/pathbuf-equivalent-to-string-is-empty/24823
    if parent.as_os_str().is_empty() {
        parent = Path::new(".");
    };

    let result = cargo_metadata::MetadataCommand::new()
        .current_dir(parent)
        .no_deps()
        .exec();

    match result {
        Err(cargo_metadata::Error::CargoMetadata { stderr })
            if stderr.contains("could not find `Cargo.toml`") => {}
        _ => {
            let metadata = result?;

            let value = config::try_init_with_metadata(&metadata)?;

            if let Some(s) = &value {
                sess.parse_sess()
                    .file_depinfo
                    .lock()
                    .insert(Symbol::intern(s));
            }
        }
    }

    Ok(())
}

#[rustversion::before(2023-01-19)]
fn local_crate_source_file(sess: &rustc_session::Session) -> Option<PathBuf> {
    sess.local_crate_source_file.clone()
}

// smoelius: Relevant PR and merge commit:
// - https://github.com/rust-lang/rust/pull/106810
// - https://github.com/rust-lang/rust/commit/65d2f2a5f9c323c88d1068e8e90d0b47a20d491c
#[rustversion::all(since(2023-01-19), before(2024-03-29))]
fn local_crate_source_file(sess: &rustc_session::Session) -> Option<PathBuf> {
    sess.local_crate_source_file()
}

// smoelius: Relevant PR and merge commit:
// - https://github.com/rust-lang/rust/pull/122450
// - https://github.com/rust-lang/rust/commit/685927aae69657b46323cffbeb0062835bd7fa2b
#[rustversion::since(2024-03-29)]
fn local_crate_source_file(sess: &rustc_session::Session) -> Option<PathBuf> {
    use rustc_span::RealFileName;
    sess.local_crate_source_file()
        .and_then(RealFileName::into_local_path)
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

#[rustversion::all(since(2023-06-28), before(2023-12-18))]
fn early_error(msg: impl Into<rustc_errors::DiagnosticMessage>) -> ! {
    let handler =
        rustc_session::EarlyErrorHandler::new(rustc_session::config::ErrorOutputType::default());
    handler.early_error(msg)
}

#[rustversion::all(since(2023-12-18), before(2023-12-23))]
fn early_error(msg: impl Into<rustc_errors::DiagnosticMessage>) -> ! {
    let handler =
        rustc_session::EarlyDiagCtxt::new(rustc_session::config::ErrorOutputType::default());
    handler.early_error(msg)
}

#[rustversion::all(since(2023-12-23), before(2024-03-05))]
fn early_error(msg: impl Into<rustc_errors::DiagnosticMessage>) -> ! {
    let handler =
        rustc_session::EarlyDiagCtxt::new(rustc_session::config::ErrorOutputType::default());
    handler.early_fatal(msg)
}

#[rustversion::since(2024-03-05)]
fn early_error(msg: impl Into<rustc_errors::DiagMessage>) -> ! {
    let handler =
        rustc_session::EarlyDiagCtxt::new(rustc_session::config::ErrorOutputType::default());
    handler.early_fatal(msg)
}
