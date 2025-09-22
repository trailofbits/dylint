use anyhow::{Context, Result, ensure};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[allow(clippy::module_name_repetitions)]
pub trait CommandExt {
    fn logged_output(&mut self, require_success: bool) -> Result<Output>;
    fn success(&mut self) -> Result<()>;
}

impl CommandExt for Command {
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    fn logged_output(&mut self, require_success: bool) -> Result<Output> {
        log::debug!("{:?}", self.get_envs().collect::<Vec<_>>());
        log::debug!("{:?}", self.get_current_dir());
        log::debug!("{self:?}");

        #[allow(clippy::disallowed_methods)]
        let output = self
            .output()
            .with_context(|| format!("Could not get output of `{self:?}`"))?;

        ensure!(
            !require_success || output.status.success(),
            "command failed: {:?}\nstdout: {:?}\nstderr: {:?}",
            self,
            std::str::from_utf8(&output.stdout).unwrap_or_default(),
            std::str::from_utf8(&output.stderr).unwrap_or_default()
        );

        Ok(output)
    }

    // smoelius: Why not get the status by calling `self.output()`? Because we don't want stdout and
    // stderr to be captured.
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    fn success(&mut self) -> Result<()> {
        log::debug!("{:?}", self.get_envs().collect::<Vec<_>>());
        log::debug!("{:?}", self.get_current_dir());
        log::debug!("{self:?}");

        let status = self
            .status()
            .with_context(|| format!("Could not get status of `{self:?}`"))?;

        ensure!(status.success(), "command failed: {self:?}");

        Ok(())
    }
}

#[allow(unused_variables)]
pub fn driver(toolchain: &str, driver: &Path) -> Result<Command> {
    #[allow(unused_mut)]
    let mut command = Command::new(driver);
    #[cfg(windows)]
    {
        // MinerSebas: To successfully determine the dylint driver Version on Windows,
        // it is neccesary to add some Libraries to the Path.
        let new_path = prepend_toolchain_path(toolchain)?;
        command.envs(vec![(crate::env::PATH, new_path)]);
    }
    Ok(command)
}

pub fn prepend_toolchain_path(toolchain: impl AsRef<Path>) -> Result<OsString> {
    let rustup_home = crate::env::var(crate::env::RUSTUP_HOME)?;
    prepend_path(
        Path::new(&rustup_home)
            .join("toolchains")
            .join(toolchain)
            .join("bin"),
    )
}

pub fn prepend_path(path: impl AsRef<OsStr>) -> Result<OsString> {
    let old_path = crate::env::var(crate::env::PATH)?;
    let new_path = std::env::join_paths(
        std::iter::once(PathBuf::from(path.as_ref())).chain(std::env::split_paths(&old_path)),
    )?;
    Ok(new_path)
}
