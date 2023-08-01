use ansi_term::Style;
use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use is_terminal::IsTerminal;
use std::{io::Write, path::Path, process::Stdio};

#[allow(clippy::module_name_repetitions)]
pub use home::cargo_home;

#[must_use]
pub fn build(description: &str, quiet: bool) -> crate::Command {
    cargo("build", "Building", description, quiet)
}

// smoelius: `cargo check` and `cargo fix` are never silenced.
#[must_use]
pub fn check(description: &str) -> crate::Command {
    cargo("check", "Checking", description, false)
}

// smoelius: `cargo check` and `cargo fix` are never silenced.
#[must_use]
pub fn fix(description: &str) -> crate::Command {
    cargo("fix", "Fixing", description, false)
}

#[must_use]
pub fn init(description: &str, quiet: bool) -> crate::Command {
    cargo("init", "Initializing", description, quiet)
}

#[must_use]
pub fn test(description: &str, quiet: bool) -> crate::Command {
    cargo("test", "Testing", description, quiet)
}

#[must_use]
pub fn update(description: &str, quiet: bool) -> crate::Command {
    cargo("update", "Updating", description, quiet)
}

fn cargo(subcommand: &str, verb: &str, description: &str, quiet: bool) -> crate::Command {
    if !quiet {
        // smoelius: Writing directly to `stderr` prevents capture by `libtest`.
        let message = format!("{verb} {description}");
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
    let mut command = crate::Command::new("cargo");
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
    command.args([subcommand]);
    if quiet {
        command.stderr(Stdio::null());
    }
    command
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
