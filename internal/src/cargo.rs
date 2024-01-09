use crate::CommandExt;
use ansi_term::Style;
use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use is_terminal::IsTerminal;
use once_cell::sync::Lazy;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[allow(clippy::module_name_repetitions)]
pub use home::cargo_home;

static STABLE_CARGO: Lazy<PathBuf> = Lazy::new(|| {
    let mut command = Command::new("rustup");
    command.args(["+stable", "which", "cargo"]);
    let output = command.logged_output(true).unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    PathBuf::from(stdout.trim_end())
});

/// A `cargo` command builder
///
/// Note that [`std::process::Command`]is itself a builder. So technically that makes this a
/// "builder builder".
pub struct Builder {
    subcommand: String,
    verb: String,
    description: String,
    stable: bool,
    quiet: bool,
}

#[must_use]
pub fn build(description: &str) -> Builder {
    Builder::new("build", "Building", description)
}

#[must_use]
pub fn check(description: &str) -> Builder {
    Builder::new("check", "Checking", description)
}

#[must_use]
pub fn fetch(description: &str) -> Builder {
    Builder::new("fetch", "Fetching", description)
}

#[must_use]
pub fn fix(description: &str) -> Builder {
    Builder::new("fix", "Fixing", description)
}

#[must_use]
pub fn init(description: &str) -> Builder {
    Builder::new("init", "Initializing", description)
}

#[must_use]
pub fn run(description: &str) -> Builder {
    Builder::new("run", "Running", description)
}

#[must_use]
pub fn test(description: &str) -> Builder {
    Builder::new("test", "Testing", description)
}

#[must_use]
pub fn update(description: &str) -> Builder {
    Builder::new("update", "Updating", description)
}

impl Builder {
    fn new(subcommand: &str, verb: &str, description: &str) -> Self {
        Self {
            subcommand: subcommand.to_owned(),
            verb: verb.to_owned(),
            description: description.to_owned(),
            quiet: false,
            stable: false,
        }
    }

    /// Whether to allow the command to write to standard error.
    pub fn quiet(&mut self, value: bool) -> &mut Self {
        // smoelius: `cargo check` and `cargo fix` are never silenced.
        if value {
            assert!(!matches!(self.subcommand.as_str(), "check" | "fix"));
        }
        self.quiet = value;
        self
    }

    /// Whether to use a cached path to stable `cargo`. Using the cached path avoids repeated calls
    /// to `rustup`.
    pub fn stable(&mut self, value: bool) -> &mut Self {
        self.stable = value;
        self
    }

    /// Consumes the builder and returns a [`std::process::Command`].
    pub fn build(&mut self) -> Command {
        if !self.quiet {
            // smoelius: Writing directly to `stderr` prevents capture by `libtest`.
            let message = format!("{} {}", self.verb, self.description);
            std::io::stderr()
                .write_fmt(format_args!(
                    "{}\n",
                    if std::io::stderr().is_terminal() {
                        Style::new().bold()
                    } else {
                        Style::new()
                    }
                    .paint(message)
                ))
                .expect("Could not write to stderr");
        }
        let mut command = if self.stable {
            Command::new(&*STABLE_CARGO)
        } else {
            Command::new("cargo")
        };
        #[cfg(windows)]
        {
            // smoelius: Work around: https://github.com/rust-lang/rustup/pull/2978
            let cargo_home = cargo_home().unwrap();
            let old_path = crate::env::var(crate::env::PATH).unwrap();
            let new_path = std::env::join_paths(
                std::iter::once(Path::new(&cargo_home).join("bin"))
                    .chain(std::env::split_paths(&old_path)),
            )
            .unwrap();
            command.envs(vec![(crate::env::PATH, new_path)]);
        }
        command.args([&self.subcommand]);
        if self.quiet {
            command.stderr(Stdio::null());
        }
        command
    }
}

/// Get metadata based on the current directory.
pub fn current_metadata() -> Result<Metadata> {
    MetadataCommand::new().no_deps().exec().map_err(Into::into)
}

pub fn package_with_root(metadata: &Metadata, package_root: &Path) -> Result<Package> {
    let packages = metadata
        .packages
        .iter()
        .map(|package| {
            let path = package
                .manifest_path
                .parent()
                .ok_or_else(|| anyhow!("Could not get parent directory"))?;
            Ok(if path == package_root {
                Some(package)
            } else {
                None
            })
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>>>()?;

    ensure!(
        packages.len() <= 1,
        "Found multiple packages in `{}`",
        package_root.to_string_lossy()
    );

    packages
        .into_iter()
        .next()
        .cloned()
        .ok_or_else(|| anyhow!("Found no packages in `{}`", package_root.to_string_lossy()))
}

pub fn package(metadata: &Metadata, package_id: &PackageId) -> Result<Package> {
    metadata
        .packages
        .iter()
        .find(|package| package.id == *package_id)
        .cloned()
        .ok_or_else(|| anyhow!("Could not find package"))
}
