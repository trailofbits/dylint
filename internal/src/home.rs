use std::{env, path::PathBuf};

#[must_use]
pub fn cargo_home() -> Option<PathBuf> {
    if let Ok(cargo_home) = env::var(crate::env::CARGO_HOME) {
        Some(PathBuf::from(cargo_home))
    } else {
        env::home_dir().map(|path| path.join(".cargo"))
    }
}
