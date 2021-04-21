use anyhow::{anyhow, ensure, Result};
use dylint_internal::{
    env::{self, var},
    Command,
};
use semver::{Version, VersionReq};
use std::{
    fs::{copy, create_dir_all, write},
    path::{Path, PathBuf},
};
use tempfile::tempdir;

const README_TXT: &str = r#"
This directory contains Rust compiler drivers used by Dylint
(https://github.com/trailofbits/dylint).

Deleting this directory will cause Dylint to rebuild the drivers
the next time it needs them, but will have no ill effects.
"#;

fn cargo_toml(toolchain: &str, dylint_driver_spec: &str) -> String {
    format!(
        r#"
[package]
name = "dylint_driver-{}"
version = "0.1.0"
edition = "2018"

[dependencies]
anyhow = "1.0.38"
env_logger = "0.8.3"
dylint_driver = {{ {} }}
"#,
        toolchain, dylint_driver_spec,
    )
}

const MAIN_RS: &str = r#"
use anyhow::Result;
use std::env;
use std::ffi::OsString;

pub fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<_> = env::args().map(OsString::from).collect();

    dylint_driver::dylint_driver(&args)
}
"#;

#[allow(unknown_lints)]
#[allow(question_mark_in_expression)]
pub fn get(toolchain: &str) -> Result<PathBuf> {
    let dylint_drivers = dylint_drivers()?;

    let driver_dir = dylint_drivers.join(&toolchain);
    if !driver_dir.is_dir() {
        create_dir_all(&driver_dir)?;
    }

    let driver = driver_dir.join("dylint-driver");
    if !driver.exists() || is_outdated(&driver)? {
        build(toolchain, &driver)?;
    }

    Ok(driver)
}

fn dylint_drivers() -> Result<PathBuf> {
    if let Ok(dylint_driver_path) = var(env::DYLINT_DRIVER_PATH) {
        let dylint_drivers = Path::new(&dylint_driver_path);
        ensure!(dylint_drivers.is_dir());
        Ok(dylint_drivers.to_path_buf())
    } else {
        let home = var(env::HOME)?;
        let dylint_drivers = Path::new(&home).join(".dylint_drivers");
        if !dylint_drivers.is_dir() {
            create_dir_all(&dylint_drivers)?;
            write(dylint_drivers.join("README.txt"), README_TXT)?;
        }
        Ok(dylint_drivers)
    }
}

fn is_outdated(driver: &Path) -> Result<bool> {
    let output = Command::new(driver).args(&["-V"]).output()?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    let theirs = stdout
        .trim_end()
        .rsplitn(2, ' ')
        .next()
        .ok_or_else(|| anyhow!("Could not parse driver version"))?;

    let their_version = Version::parse(theirs)?;
    let their_req = VersionReq::parse(theirs)?;

    let our_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let our_req = VersionReq::parse(env!("CARGO_PKG_VERSION"))?;

    Ok(their_req.matches(&our_version) && !our_req.matches(&their_version))
}

#[allow(clippy::assertions_on_constants)]
#[allow(clippy::expect_used)]
fn build(toolchain: &str, driver: &Path) -> Result<()> {
    let tempdir = tempdir()?;
    let package = tempdir.path();

    let version_spec = format!("version = \"={}\"", env!("CARGO_PKG_VERSION"));

    // smoelius: Fetch the `dylint_driver` package from crates.io if built in release mode or if
    // `dylint_driver_remote` is enabled.
    #[cfg(any(not(debug_assertions), feature = "dylint_driver_remote"))]
    let path_spec = "";
    #[cfg(all(debug_assertions, not(feature = "dylint_driver_remote")))]
    let path_spec = format!(
        ", path = \"{}\"",
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Could not get parent directory")
            .join("driver")
            .to_string_lossy()
    );

    let dylint_driver_spec = format!("{}{}", version_spec, path_spec);

    write(
        package.join("Cargo.toml"),
        cargo_toml(toolchain, &dylint_driver_spec),
    )?;
    let src = package.join("src");
    create_dir_all(&src)?;
    write(&src.join("main.rs"), MAIN_RS)?;

    dylint_internal::build()
        .envs(vec![
            (env::RUSTFLAGS, "-C rpath=yes"),
            (env::RUSTUP_TOOLCHAIN, toolchain),
        ])
        .current_dir(&package)
        .success()?;

    copy(
        package
            .join("target")
            .join("debug")
            .join(format!("dylint_driver-{}", toolchain)),
        driver,
    )?;

    Ok(())
}
