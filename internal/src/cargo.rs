use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};

#[must_use]
pub fn build() -> crate::Command {
    cargo("build")
}

#[must_use]
pub fn check() -> crate::Command {
    cargo("check")
}

#[must_use]
pub fn fix() -> crate::Command {
    cargo("fix")
}

#[must_use]
pub fn test() -> crate::Command {
    cargo("test")
}

fn cargo(subcommand: &str) -> crate::Command {
    let mut command = crate::Command::new("cargo");
    command.args(&[subcommand]);
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
