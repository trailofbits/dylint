use std::{fs::read_to_string, sync::OnceLock};
use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub struct Error {
    inner: Inner,
}

impl Error {
    #[must_use]
    pub const fn other(value: String) -> Self {
        Self {
            inner: Inner::Other(value),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> From<T> for Error
where
    Inner: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            inner: Inner::from(value),
        }
    }
}

#[derive(Debug, ThisError)]
enum Inner {
    #[error("cargo metadata error: {0}")]
    CargoMetadata(#[from] cargo_metadata::Error),
    #[error("io error: {0}: {1}")]
    Io(String, std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Other(String),
}

static CONFIG_TABLE: OnceLock<toml::value::Table> = OnceLock::new();

pub fn get() -> Option<&'static toml::value::Table> {
    CONFIG_TABLE.get()
}

// smoelius: `try_init_with_metadata` returns a string so that `dylint_linting` can record it in
// `file_depinfo`.
pub fn try_init_with_metadata(metadata: &cargo_metadata::Metadata) -> Result<Option<String>> {
    if CONFIG_TABLE.get().is_some() {
        return Ok(None);
    }

    let cargo_metadata::Metadata { workspace_root, .. } = metadata;

    let dylint_toml = workspace_root.join("dylint.toml");

    let value = if dylint_toml
        .try_exists()
        .map_err(|error| Inner::Io(format!("`try_exists` failed for {dylint_toml:?}"), error))?
    {
        let value = read_to_string(&dylint_toml).map_err(|error| {
            Inner::Io(
                format!("`read_to_string` failed for {dylint_toml:?}"),
                error,
            )
        })?;
        Some(value)
    } else {
        None
    };

    if let Some(s) = &value {
        init_from_string(s)?;
    }

    Ok(value)
}

pub fn init_from_string(s: &str) -> Result<()> {
    assert!(CONFIG_TABLE.get().is_none());

    let toml: toml::Value = toml::from_str(s)?;

    let table = toml
        .as_table()
        .cloned()
        .ok_or_else(|| Inner::Other("Value is not a table".into()))?;

    // smoelius: Rewrite this function (`init_from_string`) and eliminate the next `expect` once
    // `get_or_try_init` stabilizes: https://github.com/rust-lang/rust/issues/109737
    CONFIG_TABLE
        .set(table)
        .unwrap_or_else(|error| panic!("`CONFIG_TABLE` was determined to be unset above: {error}"));

    Ok(())
}
