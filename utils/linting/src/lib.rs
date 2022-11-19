#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_session;
extern crate rustc_span;

use cargo_metadata::{Metadata, MetadataCommand};
use dylint_internal::env;
use rustc_session::Session;
use rustc_span::Symbol;
use std::{any::type_name, cell::RefCell, fs::read_to_string, sync::Mutex};
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

#[macro_export]
macro_rules! __declare_and_register_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $register_pass_method:ident, $pass:expr) => {
        $crate::dylint_library!();

        extern crate rustc_lint;
        extern crate rustc_session;

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
#[macro_export]
macro_rules! __make_late_closure {
    ($pass:expr) => {
        || Box::new($pass)
    };
}

// smoelius: Relevant PR and merge commit:
// * https://github.com/rust-lang/rust/pull/101501
// * https://github.com/rust-lang/rust/commit/87788097b776f8e3662f76627944230684b671bd
#[rustversion::since(2022-09-08)]
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
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Other(String),
}

static CONFIG_TABLE: Mutex<RefCell<Option<toml::value::Table>>> = Mutex::new(RefCell::new(None));

pub fn config_or_default<T: Default + serde::de::DeserializeOwned>(name: &str) -> T {
    config::<T>(name)
        .map(Option::unwrap_or_default)
        .expect(&format!("Could not parse config as `{}`", type_name::<T>()))
}

pub fn config<T: serde::de::DeserializeOwned>(name: &str) -> ConfigResult<Option<T>> {
    let toml = config_toml(name)?;
    toml.map(toml::Value::try_into::<T>)
        .transpose()
        .map_err(Into::into)
}

pub fn config_toml(name: &str) -> ConfigResult<Option<toml::Value>> {
    let config_table = CONFIG_TABLE.lock().unwrap();
    let config_table = config_table.borrow();
    let config_table = config_table.as_ref().ok_or_else(|| {
        ConfigErrorInner::Other(
            "Config is not initialized; `init_config` should have been called from `register_lints`"
                .into(),
        )
    })?;
    Ok(config_table.get(name).cloned())
}

pub fn init_config(sess: &Session) {
    try_init_config(sess).unwrap_or_else(|err| {
        rustc_session::early_error(
            rustc_session::config::ErrorOutputType::default(),
            &format!("could not read configuration file: {}", err),
        );
    });
}

pub fn try_init_config(sess: &Session) -> ConfigResult<()> {
    let config_table = CONFIG_TABLE.lock().unwrap();

    if config_table.borrow().is_some() {
        return Ok(());
    }

    let value = if let Ok(value) = std::env::var(env::DYLINT_TOML) {
        sess.parse_sess.env_depinfo.lock().insert((
            Symbol::intern(env::DYLINT_TOML),
            Some(Symbol::intern(&value)),
        ));
        Some(value)
    } else {
        let local_crate_source_file = sess
            .local_crate_source_file
            .as_ref()
            .ok_or_else(|| ConfigErrorInner::Other("No source file".into()))?;

        let parent = local_crate_source_file
            .parent()
            .ok_or_else(|| ConfigErrorInner::Other("Could not get parent directory".into()))?;

        let result = MetadataCommand::new().current_dir(parent).no_deps().exec();

        match result {
            Err(cargo_metadata::Error::CargoMetadata { stderr })
                if stderr.contains("could not find `Cargo.toml`") =>
            {
                None
            }
            _ => {
                let Metadata { workspace_root, .. } = result?;

                let dylint_toml = workspace_root.join("dylint.toml");

                if dylint_toml.try_exists()? {
                    let value = read_to_string(&dylint_toml)?;
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
    };

    let toml: Option<toml::Value> = value.as_deref().map(toml::from_str).transpose()?;

    let table = toml
        .map(|toml| {
            toml.as_table()
                .cloned()
                .ok_or_else(|| ConfigErrorInner::Other("Value is not a table".into()))
        })
        .transpose()?;

    config_table.replace(Some(table.unwrap_or_default()));

    Ok(())
}
