use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// smoelius: `TomlDetailedDependency::unused_keys` does not appear in the original.
impl TomlDetailedDependency {
    pub fn unused_keys(&self) -> Vec<String> {
        self.other.keys().cloned().collect()
    }
}

// smoelius: `TomlDetailedDependency` was copied from:
// https://github.com/rust-lang/cargo/blob/e476251168fab96ae3c7544ee1a9f3ae3b7f885f/src/cargo/util/toml/mod.rs#L250-L287

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct TomlDetailedDependency<P: Clone = String> {
    version: Option<String>,
    registry: Option<String>,
    /// The URL of the `registry` field.
    /// This is an internal implementation detail. When Cargo creates a
    /// package, it replaces `registry` with `registry-index` so that the
    /// manifest contains the correct URL. All users won't have the same
    /// registry names configured, so Cargo can't rely on just the name for
    /// crates published by other users.
    registry_index: Option<String>,
    // `path` is relative to the file it appears in. If that's a `Cargo.toml`, it'll be relative to
    // that TOML file, and if it's a `.cargo/config` file, it'll be relative to that file.
    path: Option<P>,
    git: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    features: Option<Vec<String>>,
    optional: Option<bool>,
    default_features: Option<bool>,
    #[serde(rename = "default_features")]
    default_features2: Option<bool>,
    package: Option<String>,
    public: Option<bool>,

    /// One or more of `bin`, `cdylib`, `staticlib`, `bin:<name>`.
    artifact: Option<StringOrVec>,
    /// If set, the artifact should also be a dependency
    lib: Option<bool>,
    /// A platform name, like `x86_64-apple-darwin`
    target: Option<String>,
    /// This is here to provide a way to see the "unused manifest keys" when deserializing
    #[serde(skip_serializing)]
    #[serde(flatten)]
    other: BTreeMap<String, toml::Value>,
}
