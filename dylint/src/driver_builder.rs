use crate::error::warn;
use anyhow::{anyhow, ensure, Result};
use dylint_internal::{
    env::{self, var},
    rustup::SanitizeEnvironment,
    Command,
};
use semver::Version;
#[cfg(target_os = "windows")]
use std::fs::read_dir;
use std::{
    env::consts,
    fs::{copy, create_dir_all, write},
    path::{Path, PathBuf},
    process::Stdio,
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

fn rust_toolchain(toolchain: &str) -> String {
    format!(
        r#"
[toolchain]
channel = "{}"
components = ["llvm-tools-preview", "rustc-dev"]
"#,
        toolchain,
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
pub fn get(opts: &crate::Dylint, toolchain: &str) -> Result<PathBuf> {
    let dylint_drivers = dylint_drivers()?;

    let driver_dir = dylint_drivers.join(&toolchain);
    if !driver_dir.is_dir() {
        create_dir_all(&driver_dir)?;
    }

    let driver = driver_dir.join("dylint-driver");
    if !driver.exists() || is_outdated(opts, &driver)? {
        build(opts, toolchain, &driver)?;
    }

    Ok(driver)
}

fn dylint_drivers() -> Result<PathBuf> {
    if let Ok(dylint_driver_path) = var(env::DYLINT_DRIVER_PATH) {
        let dylint_drivers = Path::new(&dylint_driver_path);
        ensure!(dylint_drivers.is_dir());
        Ok(dylint_drivers.to_path_buf())
    } else {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find HOME directory"))?;
        let dylint_drivers = Path::new(&home).join(".dylint_drivers");
        if !dylint_drivers.is_dir() {
            create_dir_all(&dylint_drivers)?;
            write(dylint_drivers.join("README.txt"), README_TXT)?;
        }
        Ok(dylint_drivers)
    }
}

fn is_outdated(opts: &crate::Dylint, driver: &Path) -> Result<bool> {
    let output = Command::new(driver).args(&["-V"]).output()?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    let theirs = stdout
        .trim_end()
        .rsplitn(2, ' ')
        .next()
        .ok_or_else(|| anyhow!("Could not parse driver version"))?;

    let result = Version::parse(theirs);

    let their_version = match result {
        Ok(version) => version,
        Err(err) => {
            warn(
                opts,
                &format!("Could not determine driver version: {}", err),
            );
            return Ok(true);
        }
    };

    let our_version = Version::parse(env!("CARGO_PKG_VERSION"))?;

    Ok(their_version < our_version)
}

#[allow(clippy::assertions_on_constants)]
#[allow(clippy::expect_used)]
fn build(opts: &crate::Dylint, toolchain: &str, driver: &Path) -> Result<()> {
    let tempdir = tempdir()?;
    let package = tempdir.path();

    let version_spec = format!("version = \"={}\"", env!("CARGO_PKG_VERSION"));

    // smoelius: Assume the `dylint_driver` package is local if building in debug mode and if
    // `dylint_driver_local` is enabled.
    #[cfg(any(not(debug_assertions), not(feature = "dylint_driver_local")))]
    let path_spec = "";
    #[cfg(all(debug_assertions, feature = "dylint_driver_local"))]
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
    write(package.join("rust-toolchain"), rust_toolchain(toolchain))?;
    let src = package.join("src");
    create_dir_all(&src)?;
    write(&src.join("main.rs"), MAIN_RS)?;

    let mut command = dylint_internal::build();
    command
        .sanitize_environment()
        .envs(vec![(env::RUSTFLAGS, "-C rpath=yes")])
        .current_dir(&package);
    if opts.quiet {
        command.stderr(Stdio::null());
    }
    command.success()?;

    copy(
        package.join("target").join("debug").join(format!(
            "dylint_driver-{}{}",
            toolchain,
            consts::EXE_SUFFIX
        )),
        driver,
    )?;

    // MinerSebas: To succesfully determine the dylint driver Version on Windows,
    // it is neccesary to place copies of the toolchain dll's next to the driver.
    #[cfg(target_os = "windows")]
    {
        let rustup_home = var(env::RUSTUP_HOME)?;
        let path = PathBuf::from(rustup_home)
            .join("toolchains")
            .join(toolchain)
            .join("bin");

        for file in read_dir(path)?.flatten() {
            let file_name = file.file_name();

            if let Some(file_name) = file_name.to_str() {
                if file_name.ends_with(consts::DLL_SUFFIX) {
                    copy(
                        file.path(),
                        dylint_drivers()?.join(toolchain).join(file_name),
                    )?;
                }
            }
        }
    }

    Ok(())
}
