use ansi_term::Style;
use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use std::{io::Write, process::Stdio};

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
pub fn test(description: &str, quiet: bool) -> crate::Command {
    cargo("test", "Testing", description, quiet)
}

#[must_use]
pub fn update(description: &str, quiet: bool) -> crate::Command {
    cargo("update", "Updating", description, quiet)
}

fn cargo(subcommand: &str, verb: &str, description: &str, quiet: bool) -> crate::Command {
    if !quiet {
        // smoelius: Writing directly to `stderr` avoids capture by `libtest`.
        let message = format!("{} {}", verb, description);
        std::io::stderr()
            .write_fmt(format_args!("{}\n", Style::new().bold().paint(message)))
            .expect("Could not write to stderr");
    }
    let mut command = crate::Command::new("cargo");
    command.args(&[subcommand]);
    if quiet {
        command.stderr(Stdio::null());
    }
    command
}

/// Get metadata based on the current directory.
pub fn current_metadata() -> Result<Metadata> {
    MetadataCommand::new().no_deps().exec().map_err(Into::into)
}

pub fn root_package(metadata: &Metadata) -> Result<Package> {
    ensure!(metadata.packages.len() <= 1, "Found multiple packages");
    metadata
        .packages
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Found no packages"))
}

pub fn package(metadata: &Metadata, package_id: &PackageId) -> Result<Package> {
    metadata
        .packages
        .iter()
        .find(|package| package.id == *package_id)
        .cloned()
        .ok_or_else(|| anyhow!("Could not find package"))
}
