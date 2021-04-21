use anyhow::{ensure, Result};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Output,
};

pub struct Command {
    envs: Vec<(OsString, OsString)>,
    command: std::process::Command,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            envs: vec![],
            command: std::process::Command::new(program),
        }
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs = vars
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_os_string(), v.as_ref().to_os_string()))
            .collect();
        self.command.envs(self.envs.clone().into_iter());
        self
    }

    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn output(&mut self) -> Result<Output> {
        log::debug!("{:?}", self.envs);
        log::debug!("{:?}", self.command);

        self.command.output().map_err(Into::into)
    }

    pub fn success(&mut self) -> Result<()> {
        log::debug!("{:?}", self.envs);
        log::debug!("{:?}", self.command);

        let status = self.command.status()?;

        ensure!(status.success(), "command failed: {:?}", self.command);

        Ok(())
    }
}
