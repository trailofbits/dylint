use anyhow::{Context, Result, bail};
use std::{
    ffi::{OsStr, OsString},
    fmt::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::NamedTempFile;

#[allow(clippy::module_name_repetitions)]
pub trait CommandExt {
    fn logged_output(&mut self, require_success: bool) -> Result<Output>;
    fn success(&mut self) -> Result<()>;
}

impl CommandExt for Command {
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_unhandled_error))]
    fn logged_output(&mut self, require_success: bool) -> Result<Output> {
        log::debug!("{:?}", self.get_envs().collect::<Vec<_>>());
        log::debug!("{:?}", self.get_current_dir());
        log::debug!("{self:?}");

        #[allow(clippy::disallowed_methods)]
        let output = self
            .output()
            .with_context(|| format!("Could not get output of `{self:?}`"))?;

        if require_success && !output.status.success() {
            let msg = failure_message(self, Some(&output));
            bail!("{msg}");
        }

        Ok(output)
    }

    // smoelius: Why not get the status by calling `self.output()`? Because we don't want stdout and
    // stderr to be captured.
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_unhandled_error))]
    fn success(&mut self) -> Result<()> {
        log::debug!("{:?}", self.get_envs().collect::<Vec<_>>());
        log::debug!("{:?}", self.get_current_dir());
        log::debug!("{self:?}");

        let status = self
            .status()
            .with_context(|| format!("Could not get status of `{self:?}`"))?;

        if !status.success() {
            let msg = failure_message(self, None);
            bail!("{msg}");
        }

        Ok(())
    }
}

fn failure_message(command: &Command, output: Option<&Output>) -> String {
    let mut msg = format!("`{}` command failed", command.get_program().display());
    let and_output = output.map(|_| " and output").unwrap_or_default();
    if crate::env::enabled(crate::env::CI) {
        msg.push('\n');
        // smoelius: Use `unwrap` when writing to a string.
        writeln!(msg, "::group::Full command{and_output}").unwrap();
        write_command_and_output_fmt(&mut msg, command, output).unwrap();
        msg.push_str("::endgroup::");
    } else {
        msg.push_str("; ");
        let result = (|| -> Result<PathBuf> {
            let mut tempfile =
                NamedTempFile::new().with_context(|| "Could not create named temp file")?;
            write_command_and_output_io(&mut tempfile, command, output)?;
            tempfile.keep().map(|(_, path)| path).map_err(Into::into)
        })();
        // smoelius: Panic if error propagation fails.
        match result {
            Ok(path) => write!(
                &mut msg,
                "the full command{and_output} can be found in `{}`",
                path.display()
            ),
            Err(error) => write!(
                &mut msg,
                "could not write command{and_output} to tempfile: {error}"
            ),
        }
        .unwrap();
    }
    msg
}

macro_rules! impl_write_command_and_output {
    ($ident:ident) => {
        pastey::paste! {
            fn [< write_command_and_output_ $ident >](
                mut stream: impl std::$ident::Write,
                command: &Command,
                output: Option<&Output>,
            ) -> Result<()> {
                writeln!(stream, "{command:?}")
                    .and_then(|()| {
                        if let Some(output) = output {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            writeln!(
                                stream,
                                "stdout:\n```\n{}\n```\nstderr:\n```\n{}\n```",
                                stdout.trim_end_matches('\n'),
                                stderr.trim_end_matches('\n'),
                            )
                        } else {
                            Ok(())
                        }
                    })
                    .with_context(|| "Could not write to stream")
            }
        }
    };
}

impl_write_command_and_output!(fmt);
impl_write_command_and_output!(io);

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
