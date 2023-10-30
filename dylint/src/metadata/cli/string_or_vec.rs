#![allow(clippy::doc_markdown, clippy::use_self)]
#![cfg_attr(
    dylint_lib = "inconsistent_qualification",
    allow(inconsistent_qualification)
)]
#![cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]

use serde::{de, Serialize};
use serde_untagged::UntaggedEnumVisitor;

// smoelius: `StringOrVec` was copied from:
// https://github.com/rust-lang/cargo/blob/e476251168fab96ae3c7544ee1a9f3ae3b7f885f/src/cargo/util/toml/mod.rs#L871-L887

/// A StringOrVec can be parsed from either a TOML string or array,
/// but is always stored as a vector.
#[derive(Clone, Debug, Serialize, Eq, PartialEq, PartialOrd, Ord)]
pub struct StringOrVec(Vec<String>);

impl<'de> de::Deserialize<'de> for StringOrVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        UntaggedEnumVisitor::new()
            .expecting("string or list of strings")
            .string(|value| Ok(StringOrVec(vec![value.to_owned()])))
            .seq(|value| value.deserialize().map(StringOrVec))
            .deserialize(deserializer)
    }
}
