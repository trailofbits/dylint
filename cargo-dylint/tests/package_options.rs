use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use dylint_internal::rustup::SanitizeEnvironment;
use predicates::prelude::*;
use regex::Regex;
use semver::Version;
use std::{fs::read_to_string, path::Path};
use tempfile::tempdir;
use test_log::test;

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path = tempdir.path().join("filled_in");

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&["dylint", "--new", &path.to_string_lossy(), "--isolate"])
        .assert()
        .success();

    dylint_internal::build("filled-in dylint-template", false)
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();

    dylint_internal::test("filled-in dylint-template", false)
        .sanitize_environment()
        .current_dir(&path)
        .success()
        .unwrap();
}

#[test]
fn downgrade_upgrade_package() {
    let tempdir = tempdir().unwrap();

    dylint_internal::clone_dylint_template(tempdir.path()).unwrap();

    let mut rust_version = rust_version(tempdir.path()).unwrap();
    assert!(rust_version.minor != 0);
    rust_version.minor -= 1;

    let upgrade = || {
        let mut command = std::process::Command::cargo_bin("cargo-dylint").unwrap();
        command.args(&[
            "dylint",
            "--upgrade",
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

    upgrade().args(&["--force"]).assert().success();

    dylint_internal::build("downgraded dylint-template", false)
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    dylint_internal::test("downgraded dylint-template", false)
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    if cfg!(not(unix)) {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(&["dylint", "--upgrade", &tempdir.path().to_string_lossy()])
            .assert()
            .success();
    } else {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(&[
                "dylint",
                "--upgrade",
                &tempdir.path().to_string_lossy(),
                "--bisect",
            ])
            .assert()
            .success();

        dylint_internal::build("upgraded dylint-template", false)
            .sanitize_environment()
            .current_dir(tempdir.path())
            .success()
            .unwrap();

        dylint_internal::test("upgraded dylint-template", false)
            .sanitize_environment()
            .current_dir(tempdir.path())
            .success()
            .unwrap();
    }
}

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
