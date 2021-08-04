use crate::{env, Command};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub trait SanitizeEnvironment {
    fn sanitize_environment(&mut self) -> &mut Self;
}

impl SanitizeEnvironment for crate::Command {
    fn sanitize_environment(&mut self) -> &mut Self {
        self.env_remove(env::RUSTUP_TOOLCHAIN);
        self
    }
}

pub fn toolchain_path(path: &Path) -> Result<PathBuf> {
    let output = Command::new("rustup")
        .sanitize_environment()
        .current_dir(path)
        .args(&["which", "rustc"])
        .output()?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    let path = PathBuf::from(stdout);
    // smoelius: `path` should end with `/bin/rustc`.
    path.ancestors()
        .nth(2)
        .map(Into::into)
        .ok_or_else(|| anyhow!("Could not get ancestor"))
}
