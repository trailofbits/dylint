use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_internal::rustup::SanitizeEnvironment;
use predicates::prelude::*;
use regex::Regex;
use semver::Version;
use std::{fs::read_to_string, path::Path};
use tempfile::tempdir;
use test_log::test;

const RUST_VERSION: &str = "1.63.0";

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path = tempdir.path().join("filled_in");

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&["dylint", "new", &path.to_string_lossy(), "--isolate"])
        .assert()
        .success();

    check_dylint_dependencies(&path).unwrap();

    dylint_internal::packaging::use_local_packages(&path).unwrap();

    dylint_internal::cargo::build("filled-in dylint-template", false)
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();

    dylint_internal::cargo::test("filled-in dylint-template", false)
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();
}

fn check_dylint_dependencies(path: &Path) -> Result<()> {
    let metadata = MetadataCommand::new().current_dir(path).no_deps().exec()?;
    for package in metadata.packages {
        for Dependency { name: dep, req, .. } in &package.dependencies {
            if dep.starts_with("dylint") {
                assert_eq!("^".to_owned() + env!("CARGO_PKG_VERSION"), req.to_string());
            }
        }
    }
    Ok(())
}

#[test]
fn downgrade_upgrade_package() {
    let tempdir = tempdir().unwrap();

    dylint_internal::testing::new_template(tempdir.path()).unwrap();

    // smoelius: I broke this downgrading code when I switched dylint-template from using a git tag
    // to a git revision to refer to `clippy_utils`. For now, just hardcode the downgrade version.
    /* let mut rust_version = rust_version(tempdir.path()).unwrap();
    assert!(rust_version.minor != 0);
    rust_version.minor -= 1; */
    let rust_version = Version::parse(RUST_VERSION).unwrap();

    let upgrade = || {
        let mut command = std::process::Command::cargo_bin("cargo-dylint").unwrap();
        command.args(&[
            "dylint",
            "upgrade",
            &tempdir.path().to_string_lossy(),
            "--rust-version",
            &rust_version.to_string(),
        ]);
        command
    };

    upgrade()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to downgrade toolchain"));

    upgrade().args(&["--allow-downgrade"]).assert().success();

    dylint_internal::cargo::build("downgraded dylint-template", false)
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    dylint_internal::cargo::test("downgraded dylint-template", false)
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    if cfg!(not(unix)) {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(&["dylint", "upgrade", &tempdir.path().to_string_lossy()])
            .assert()
            .success();
    } else {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(&[
                "dylint",
                "upgrade",
                &tempdir.path().to_string_lossy(),
                "--bisect",
            ])
            .assert()
            .success();

        dylint_internal::cargo::build("upgraded dylint-template", false)
            .sanitize_environment()
            .current_dir(tempdir.path())
            .success()
            .unwrap();

        dylint_internal::cargo::test("upgraded dylint-template", false)
            .sanitize_environment()
            .current_dir(tempdir.path())
            .success()
            .unwrap();
    }
}

#[allow(dead_code)]
fn rust_version(path: &Path) -> Result<Version> {
    let re = Regex::new(r#"^clippy_utils = .*\btag = "rust-([^"]*)""#).unwrap();
    let manifest = path.join("Cargo.toml");
    let file = read_to_string(&manifest).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            manifest.to_string_lossy()
        )
    })?;
    let rust_version = file
        .lines()
        .find_map(|line| re.captures(line).map(|captures| captures[1].to_owned()))
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))?;
    Version::parse(&rust_version).map_err(Into::into)
}
