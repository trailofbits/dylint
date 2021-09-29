use crate::{
    cargo::{current_metadata, package},
    sed::find_and_replace,
};
use anyhow::{anyhow, Result};
use std::{fs::OpenOptions, io::Write, path::Path};

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";

const DYLINT_TEMPLATE_REV: &str = "84703d471eb492792b5bb6f845bc343244211401";

pub fn clone_dylint_template(path: &Path) -> Result<()> {
    crate::checkout(DYLINT_TEMPLATE_URL, DYLINT_TEMPLATE_REV, path)?;
    isolate(path)?;
    use_local_packages(path)?;
    allow_unused_extern_crates(path)?;
    Ok(())
}

// smoelius: If a package is checked out in the current directory, this must be dealt with:
// error: current package believes it's in a workspace when it's not
pub fn isolate(path: &Path) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.join("Cargo.toml"))?;

    writeln!(file, "[workspace]")?;

    Ok(())
}

// smoelius: If you clone, say, `dylint-template` and run `cargo test` on it, it will obtain Dylint
// packages from `crates.io`. But for the tests in this repository, you often want it to use the
// packages in this repository. The function `use_local_packages` patches a workspace's `Cargo.toml`
// file to do so.
fn use_local_packages(path: &Path) -> Result<()> {
    let metadata = current_metadata()?;

    let manifest = path.join("Cargo.toml");

    find_and_replace(
        &manifest,
        &[
            r#"s/(?m)^dylint_testing = "([^"]*)"/dylint_testing = { version = "${1}", features = ["dylint_driver_local"] }/"#,
        ],
    )?;

    let mut file = OpenOptions::new().write(true).append(true).open(manifest)?;

    writeln!(file, "[patch.crates-io]")?;

    for package_id in &metadata.workspace_members {
        let package = package(&metadata, package_id)?;
        let path = package
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent"))?;
        writeln!(
            file,
            r#"{} = {{ path = "{}" }}"#,
            package.name,
            path.to_string().replace('\\', "\\\\")
        )?;
    }

    Ok(())
}

fn allow_unused_extern_crates(path: &Path) -> Result<()> {
    find_and_replace(
        &path.join("src").join("lib.rs"),
        &[r#"s/(?m)^#!\[warn\(unused_extern_crates\)\]\r?\n//"#],
    )
}
