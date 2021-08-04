use crate::{
    cargo::{metadata, package},
    Command,
};
use anyhow::{anyhow, Result};
use std::{fs::OpenOptions, io::Write, path::Path};

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";

const DYLINT_TEMPLATE_REV: &str = "6d6f2858874b254e41a26bab388eab0406a7e5e7";

pub fn checkout_dylint_template(path: &Path) -> Result<()> {
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
    let metadata = metadata()?;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.join("Cargo.toml"))?;

    writeln!(file, "[patch.crates-io]")?;

    for package_id in &metadata.workspace_members {
        let package = package(&metadata, package_id)?;
        let path = package
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent"))?;
        writeln!(file, r#"{} = {{ path = "{}" }}"#, package.name, path)?;
    }

    Ok(())
}

fn allow_unused_extern_crates(path: &Path) -> Result<()> {
    Command::new("sh")
        .current_dir(&path)
        .args(&[
            "-c",
            r#"
                find . -name '*.rs' -print0 |
                xargs -0 -n 1 sed -i -e '/^#!\[warn(unused_extern_crates)\]$/d'
            "#,
        ])
        .success()
}
