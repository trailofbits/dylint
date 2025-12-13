use crate::{error::warn, opts};
use anyhow::{Context, Result, anyhow, ensure};
use cargo_metadata::MetadataCommand;
use dylint_internal::{
    CommandExt, driver as dylint_driver, env,
    rustup::{SanitizeEnvironment, toolchain_path},
};
use semver::Version;
use std::{
    env::{consts, home_dir},
    fs::{copy, create_dir_all, rename, write},
    path::{Path, PathBuf},
};
use tempfile::{NamedTempFile, tempdir};

include!(concat!(env!("OUT_DIR"), "/dylint_driver_manifest_dir.rs"));

const README_TXT: &str = "
This directory contains Rust compiler drivers used by Dylint
(https://github.com/trailofbits/dylint).

Deleting this directory will cause Dylint to rebuild the drivers
the next time it needs them, but will have no ill effects.
";

// smoelius: We need `#![feature(rustc_private)]` as it changes `dylib` linking behavior and allows
// us to link to `rustc_driver`. See: https://github.com/rust-lang/rust/pull/122362
const MAIN_RS: &str = r"
#![feature(rustc_private)]

use anyhow::Result;
use std::env;

pub fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<_> = env::args_os().collect();

    dylint_driver::dylint_driver(&args)
}
";

#[cfg_attr(
    dylint_lib = "question_mark_in_expression",
    allow(question_mark_in_expression)
)]
pub fn get(opts: &opts::Dylint, toolchain: &str) -> Result<PathBuf> {
    let dylint_drivers = dylint_drivers()?;

    let driver_dir = dylint_drivers.join(toolchain);
    if !driver_dir.is_dir() {
        create_dir_all(&driver_dir).with_context(|| {
            format!(
                "`create_dir_all` failed for `{}`",
                driver_dir.to_string_lossy()
            )
        })?;
    }

    let driver = driver_dir.join("dylint-driver");
    if !driver.exists() || is_outdated(opts, toolchain, &driver)? {
        build(opts, toolchain, &driver_dir)?;
    }

    Ok(driver)
}

fn dylint_drivers() -> Result<PathBuf> {
    if let Ok(dylint_driver_path) = env::var(env::DYLINT_DRIVER_PATH) {
        let dylint_drivers = Path::new(&dylint_driver_path);
        ensure!(dylint_drivers.is_dir());
        Ok(dylint_drivers.to_path_buf())
    } else {
        let home = home_dir().ok_or_else(|| anyhow!("Could not find HOME directory"))?;
        let dylint_drivers = Path::new(&home).join(".dylint_drivers");
        if !dylint_drivers.is_dir() {
            create_dir_all(&dylint_drivers).with_context(|| {
                format!(
                    "`create_dir_all` failed for `{}`",
                    dylint_drivers.to_string_lossy()
                )
            })?;
            let readme_txt = dylint_drivers.join("README.txt");
            write(&readme_txt, README_TXT).with_context(|| {
                format!("`write` failed for `{}`", readme_txt.to_string_lossy())
            })?;
        }
        Ok(dylint_drivers)
    }
}

fn is_outdated(opts: &opts::Dylint, toolchain: &str, driver: &Path) -> Result<bool> {
    (|| -> Result<bool> {
        let mut command = dylint_driver(toolchain, driver)?;
        let output = command.args(["-V"]).logged_output(true)?;
        let stdout = std::str::from_utf8(&output.stdout)?;
        let theirs = stdout
            .trim_end()
            .rsplit_once(' ')
            .map(|(_, s)| s)
            .ok_or_else(|| anyhow!("Could not determine driver version"))?;

        let their_version = Version::parse(theirs)
            .with_context(|| format!("Could not parse driver version `{theirs}`"))?;

        let our_version = Version::parse(env!("CARGO_PKG_VERSION"))?;

        Ok(their_version < our_version)
    })()
    .or_else(|error| {
        warn(opts, &error.to_string());
        Ok(true)
    })
}

#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
fn build(opts: &opts::Dylint, toolchain: &str, driver_dir: &Path) -> Result<()> {
    let tempdir = tempdir().with_context(|| "`tempdir` failed")?;
    let package = tempdir.path();

    initialize(toolchain, package)?;

    let metadata = MetadataCommand::new()
        .current_dir(package)
        .no_deps()
        .exec()?;

    let toolchain_path = toolchain_path(package)?;

    // smoelius: The commented-out code was the old behavior. It would cause the driver to have
    // rpaths like `$ORIGIN/../../`... (see https://github.com/trailofbits/dylint/issues/54). The
    // new behavior causes the driver to have absolute rpaths.
    // let rustflags = "-C rpath=yes";
    let rustflags = format!(
        "{} -C link-args=-Wl,-rpath,{}/lib ",
        env::var("RUSTFLAGS").unwrap_or_default(),
        toolchain_path.to_string_lossy()
    );

    #[cfg(debug_assertions)]
    if DYLINT_DRIVER_MANIFEST_DIR.is_none() {
        warn(opts, "In debug mode building driver from `crates.io`");
    }

    dylint_internal::cargo::build(&format!("driver for toolchain `{toolchain}`"))
        .quiet(opts.quiet)
        .build()
        .sanitize_environment()
        .envs([(env::RUSTFLAGS, rustflags)])
        .current_dir(package)
        .success()?;

    let binary = metadata
        .target_directory
        .join("debug")
        .join(format!("dylint_driver-{toolchain}{}", consts::EXE_SUFFIX));

    let named_temp_file =
        NamedTempFile::new_in(driver_dir).with_context(|| "Could not create temporary file")?;

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    copy(&binary, &named_temp_file).with_context(|| {
        format!(
            "Could not copy `{binary}` to `{}`",
            named_temp_file.path().to_string_lossy()
        )
    })?;

    let driver = driver_dir.join("dylint-driver");

    // smoelius: Windows requires that the old file be moved out of the way first.
    if cfg!(target_os = "windows") {
        let temp_path = NamedTempFile::new_in(driver_dir)
            .map(NamedTempFile::into_temp_path)
            .with_context(|| "Could not create temporary file")?;
        rename(&driver, &temp_path).unwrap_or_default();
    }

    named_temp_file.persist(&driver)?;

    Ok(())
}

// smoelius: `package` is a temporary directory. So there should be no race here.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
fn initialize(toolchain: &str, package: &Path) -> Result<()> {
    let version_spec = format!("version = \"={}\"", env!("CARGO_PKG_VERSION"));

    let path_spec = DYLINT_DRIVER_MANIFEST_DIR.map_or(String::new(), |path| {
        format!(", path = \"{}\"", path.replace('\\', "\\\\"))
    });

    let dylint_driver_spec = format!("{version_spec}{path_spec}");

    let cargo_toml_path = package.join("Cargo.toml");
    write(&cargo_toml_path, cargo_toml(toolchain, &dylint_driver_spec))
        .with_context(|| format!("`write` failed for `{}`", cargo_toml_path.to_string_lossy()))?;
    let rust_toolchain_path = package.join("rust-toolchain");
    write(&rust_toolchain_path, rust_toolchain(toolchain)).with_context(|| {
        format!(
            "`write` failed for `{}`",
            rust_toolchain_path.to_string_lossy()
        )
    })?;
    let src = package.join("src");
    create_dir_all(&src)
        .with_context(|| format!("`create_dir_all` failed for `{}`", src.to_string_lossy()))?;
    let main_rs = src.join("main.rs");
    write(&main_rs, MAIN_RS)
        .with_context(|| format!("`write` failed for `{}`", main_rs.to_string_lossy()))?;

    Ok(())
}

fn cargo_toml(toolchain: &str, dylint_driver_spec: &str) -> String {
    format!(
        r#"
[package]
name = "dylint_driver-{toolchain}"
version = "0.1.0"
edition = "2018"

[dependencies]
anyhow = "1.0"
env_logger = "0.11"
dylint_driver = {{ {dylint_driver_spec} }}
"#,
    )
}

fn rust_toolchain(toolchain: &str) -> String {
    format!(
        r#"
[toolchain]
channel = "{toolchain}"
components = ["llvm-tools-preview", "rustc-dev"]
"#,
    )
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::*;

    // smoelius: `tempdir` is a temporary directory. So there should be no race here.
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn nightly() {
        let tempdir = tempdir().unwrap();
        build(&opts::Dylint::default(), "nightly", tempdir.path()).unwrap();
    }

    // smoelius: As mentioned above, `tempdir` is a temporary directory. So there should be no race
    // here.
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    // smoelius: This test passes on macOS but for the wrong reason. On recent macOS versions (e.g.,
    // Tahoe), if you copy `/bin/sleep` to you local directory and run it, it will be killed, even
    // without `child.kill()`. I haven't yet figured out how best to address this.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn can_install_while_driver_is_running() {
        use std::process::{Command, ExitStatus};

        const WHICH: &str = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        let tempdir = tempdir().unwrap();
        let driver = tempdir.path().join("dylint-driver");

        // Set tmpdir/dylint-driver to `sleep` and call it with `infinity`.
        let stdout = Command::new(WHICH)
            .arg("sleep")
            .logged_output(false)
            .unwrap()
            .stdout;
        let sleep_path = String::from_utf8(stdout).unwrap();
        copy(sleep_path.trim_end(), &driver).unwrap();
        let mut child = Command::new(driver).arg("infinity").spawn().unwrap();

        // Install should not fail with "text file busy".
        build(&opts::Dylint::default(), "nightly", tempdir.path()).unwrap();

        child.kill().unwrap();
        let _: ExitStatus = child.wait().unwrap();
    }
}
