use crate::{env, Command};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub trait SanitizeEnvironment {
    fn sanitize_environment(&mut self) -> &mut Self;
}

impl SanitizeEnvironment for crate::Command {
    fn sanitize_environment(&mut self) -> &mut Self {
        self.env_remove(env::RUSTFLAGS);
        self.env_remove(env::RUSTUP_TOOLCHAIN);
        self
    }
}

// smoelius: Consider carefully whether you need to call this function! In most cases, the toolchain
// you want is not the one returned by rustup.
pub fn active_toolchain(path: &Path) -> Result<String> {
    let output = Command::new("rustup")
        .sanitize_environment()
        .current_dir(path)
        .args(&["show", "active-toolchain"])
        .output()?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    stdout
        .splitn(2, ' ')
        .next()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine active toolchain"))
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
