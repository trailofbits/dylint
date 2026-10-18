//! Exercises missing-driver refusal through the real CLI without external tools.

use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::env;
use predicates::prelude::*;
use std::{env::consts, fs};
use tempfile::{Builder, tempdir, tempdir_in};

#[test]
fn missing_driver_refusal_reaches_get_without_starting_a_build() -> std::io::Result<()> {
    let tempdir = tempdir()?;
    let library = Builder::new()
        .prefix(&format!("{}refusal", consts::DLL_PREFIX))
        .suffix(&format!("@no-driver-build-test{}", consts::DLL_SUFFIX))
        .tempfile_in(&tempdir)?;
    let empty_path = tempdir_in(&tempdir)?;

    // leynos: Listing an explicitly named library reaches get before loading the library
    // or querying Cargo metadata. No executable is available if this regresses
    // into the build path: CARGO and PATH are scoped only to this child process.
    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .env(env::DYLINT_DRIVER_PATH, tempdir.path())
        .env("CARGO", empty_path.path().join("cargo"))
        .env("PATH", empty_path.path())
        .args([
            "dylint",
            "list",
            "--no-driver-build",
            "--no-metadata",
            "--lib-path",
        ])
        .arg(library.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Compiler driver")
                .and(predicate::str::contains("is missing"))
                .and(predicate::str::contains(
                    "--no-driver-build requires a prebuilt compatible driver",
                )),
        );

    assert!(
        !tempdir
            .path()
            .join("no-driver-build-test/dylint-driver")
            .exists(),
        "refusal must not create a driver",
    );
    let entries = fs::read_dir(&empty_path)?;
    assert_eq!(
        0,
        entries.count(),
        "no tool should be provisioned in the isolated executable directory",
    );
    Ok(())
}
