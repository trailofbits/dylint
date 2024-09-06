use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_internal::{rustup::SanitizeEnvironment, CommandExt};
use predicates::prelude::*;
use regex::Regex;
use semver::Version;
use std::{fs::read_to_string, path::Path};
use tempfile::tempdir;

// smoelius: I expected `git2-0.17.2` to build with nightly-2022-06-30, which corresponds to
// `--rust-version 1.64.0`. I'm not sure why it doesn't.
// smoelius: Dylint's MSRV was recently bumped to 1.68.
// smoelius: `home v0.5.9` (2013-12-15) requires rustc 1.70.0 or newer.
// smoelius: `cargo-util v0.2.7` requires rustc 1.72.0 or newer.
// smoelius: `cargo-platform v0.1.8` requires rustc 1.73 or newer.
// smoelius: `rustfix v0.8.4` requires rustc 1.75 or newer.
// smoelius: `rustfix v0.8.5` requires rustc 1.77 or newer.
const RUST_VERSION: &str = "1.77.0";

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path_buf = tempdir.path().join("filled_in");

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(["dylint", "new", &path_buf.to_string_lossy(), "--isolate"])
        .assert()
        .success();

    check_dylint_dependencies(&path_buf).unwrap();

    dylint_internal::packaging::use_local_packages(&path_buf).unwrap();

    dylint_internal::cargo::build("filled-in dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&path_buf)
        .success()
        .unwrap();

    dylint_internal::cargo::test("filled-in dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&path_buf)
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

#[cfg_attr(dylint_lib = "supplementary", allow(commented_code))]
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
        command.args([
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

    upgrade().args(["--allow-downgrade"]).assert().success();

    dylint_internal::cargo::build("downgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    dylint_internal::cargo::test("downgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(["dylint", "upgrade", &tempdir.path().to_string_lossy()])
        .assert()
        .success();

    // smoelius: Temporarily disable the rest of this test because of:
    // https://github.com/dtolnay/proc-macro2/issues/451
    if cfg!(all()) {
        return;
    }

    dylint_internal::cargo::build("upgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    dylint_internal::cargo::test("upgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();
}

#[allow(dead_code)]
fn rust_version(path: &Path) -> Result<Version> {
    let re = Regex::new(r#"^clippy_utils = .*\btag = "rust-([^"]*)""#).unwrap();
    let manifest = path.join("Cargo.toml");
    let contents = read_to_string(&manifest).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            manifest.to_string_lossy()
        )
    })?;
    let rust_version = contents
        .lines()
        .find_map(|line| re.captures(line).map(|captures| captures[1].to_owned()))
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))?;
    Version::parse(&rust_version).map_err(Into::into)
}
