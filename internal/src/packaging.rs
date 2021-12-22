use crate::{
    cargo::{current_metadata, package},
    sed::find_and_replace,
};
use anyhow::{anyhow, Context, Result};
use std::{fs::OpenOptions, io::Write, path::Path};

// smoelius: If a package is checked out in the current directory, this must be dealt with:
// error: current package believes it's in a workspace when it's not
pub fn isolate(path: &Path) -> Result<()> {
    let manifest = path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    writeln!(file)
        .and_then(|_| writeln!(file, "[workspace]"))
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;

    Ok(())
}

// smoelius: If you clone, say, `dylint-template` and run `cargo test` on it, it will obtain Dylint
// packages from `crates.io`. But for the tests in this repository, you often want it to use the
// packages in this repository. The function `use_local_packages` patches a workspace's `Cargo.toml`
// file to do so.
pub fn use_local_packages(path: &Path) -> Result<()> {
    let metadata = current_metadata()?;

    let manifest = path.join("Cargo.toml");

    find_and_replace(
        &manifest,
        &[
            r#"s/(?m)^dylint_testing = "([^"]*)"/dylint_testing = { version = "${1}", features = ["dylint_driver_local"] }/"#,
        ],
    )?;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    writeln!(file)
        .and_then(|_| writeln!(file, "[patch.crates-io]"))
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;

    for package_id in &metadata.workspace_members {
        let package = package(&metadata, package_id)?;
        if package
            .targets
            .iter()
            .all(|target| target.kind.iter().all(|kind| kind != "lib"))
        {
            continue;
        }
        let path = package
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent directory"))?;
        writeln!(
            file,
            r#"{} = {{ path = "{}" }}"#,
            package.name,
            path.to_string().replace('\\', "\\\\")
        )
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;
    }

    Ok(())
}

pub fn allow_unused_extern_crates(path: &Path) -> Result<()> {
    find_and_replace(
        &path.join("src").join("lib.rs"),
        &[r#"s/(?m)^#!\[warn\(unused_extern_crates\)\]\r?\n//"#],
    )
}
