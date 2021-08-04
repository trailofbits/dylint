use crate::env::{self, var};
use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use std::path::Path;

#[must_use]
pub fn build() -> crate::Command {
    cargo("build")
}

#[must_use]
pub fn check() -> crate::Command {
    cargo("check")
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

pub fn metadata() -> Result<Metadata> {
    let manifest_dir = var(env::CARGO_MANIFEST_DIR)?;
    let manifest_path = Path::new(&manifest_dir).join("Cargo.toml");
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .map_err(Into::into)
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
