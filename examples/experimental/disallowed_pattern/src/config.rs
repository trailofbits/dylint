use crate::{
    Pattern,
    pattern::{CompiledPattern, UncompiledPattern},
};
use anyhow::Result;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

pub struct Config<Pattern> {
    pub type_pattern_map: BTreeMap<Type, Vec<Pattern>>,
}

impl Default for Config<UncompiledPattern> {
    fn default() -> Self {
        Self {
            type_pattern_map: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize)]
struct PatternsWithOptionalTypes {
    patterns: Vec<PatternWithOptionalType>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub enum Type {
    Arm,
    Block,
    #[default]
    Expr,
    Stmt,
}

#[derive(Deserialize)]
struct PatternWithOptionalType {
    #[serde(rename = "type")]
    type_: Option<Type>,
    #[serde(flatten)]
    pattern: UncompiledPattern,
}

impl<'de> Deserialize<'de> for Config<UncompiledPattern> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut type_pattern_map = BTreeMap::<_, Vec<_>>::new();
        let PatternsWithOptionalTypes {
            patterns: patterns_with_types,
        } = PatternsWithOptionalTypes::deserialize(deserializer)?;
        for PatternWithOptionalType { type_, pattern } in patterns_with_types {
            type_pattern_map
                .entry(type_.unwrap_or_default())
                .or_default()
                .push(pattern);
        }
        Ok(Config { type_pattern_map })
    }
}

impl Config<UncompiledPattern> {
    /// # Panics
    ///
    /// Panics if a pattern cannot be parsed, or if a predicate or callback cannot be compiled.
    pub fn compile(self) -> Config<CompiledPattern> {
        let type_pattern_map = self
            .type_pattern_map
            .into_iter()
            .map(|(type_, patterns)| {
                let patterns = patterns.into_iter().map(Pattern::compile).collect();
                (type_, patterns)
            })
            .collect();
        Config { type_pattern_map }
    }
}

impl Config<CompiledPattern> {
    pub fn get_slice(&self, type_: Type) -> Option<&[CompiledPattern]> {
        self.type_pattern_map.get(&type_).map(Vec::as_slice)
    }
}
