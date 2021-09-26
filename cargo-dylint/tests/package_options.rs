use anyhow::{anyhow, Result};
use assert_cmd::prelude::*;
use dylint_internal::rustup::SanitizeEnvironment;
use regex::Regex;
use semver::Version;
use std::{fs::read_to_string, path::Path};
use tempfile::tempdir;

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path = tempdir.path().join("filled_in");

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&[
            "dylint",
            "--new",
            &path.to_string_lossy().to_string(),
            "--isolate",
        ])
        .assert()
        .success();

    dylint_internal::build()
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();

    dylint_internal::test()
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();
}

#[test]
fn upgrade_package() {
    let tempdir = tempdir().unwrap();

    dylint_internal::clone_dylint_template(tempdir.path()).unwrap();

    let mut rust_version = rust_version(tempdir.path()).unwrap();
    assert!(rust_version.minor != 0);
    rust_version.minor -= 1;

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&[
            "dylint",
            "--upgrade",
            &tempdir.path().to_string_lossy().to_string(),
            "--rust-version",
            &rust_version.to_string(),
        ])
        .assert()
        .success();

    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    dylint_internal::test()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&[
            "dylint",
            "--upgrade",
            &tempdir.path().to_string_lossy().to_string(),
        ])
        .assert()
        .success();

    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    dylint_internal::test()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();
}

fn rust_version(path: &Path) -> Result<Version> {
    let re = Regex::new(r#"^clippy_utils = .*\btag = "rust-([^"]*)""#).unwrap();
    let file = read_to_string(path.join("Cargo.toml"))?;
    let rust_version = file
        .lines()
        .find_map(|line| re.captures(line).map(|captures| captures[1].to_owned()))
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))?;
    Version::parse(&rust_version).map_err(Into::into)
}
