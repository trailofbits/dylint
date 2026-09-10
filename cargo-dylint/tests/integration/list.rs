// smoelius: As of version 0.1.14, `cargo-llvm-cov` no longer sets `CARGO_TARGET_DIR`. So it is now
// safe to run these tests under `cargo-llvm-cov`.
// #![cfg(not(coverage))]

use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{
    CommandExt,
    clippy_utils::{set_clippy_utils_dependency_revision, set_toolchain_channel},
    env,
    link::{debug_path_with_active_toolchain, debug_path_with_toolchain},
    msrv,
    rustup::{SanitizeEnvironment, active_toolchain},
    testing::new_template,
};
use predicates::prelude::*;
use std::{
    env::join_paths,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

#[test]
fn one_name_multiple_toolchains() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    patch_dylint_template(
        tempdir.path(),
        msrv::MSRV_CHANNEL,
        msrv::MSRV_CLIPPY_UTILS_REV,
    )
    .unwrap();
    let msrv_toolchain = active_toolchain(tempdir.path()).unwrap();
    dylint_internal::cargo::build(&format!(
        "dylint-template with channel `{}`",
        msrv::MSRV_CHANNEL
    ))
    .build()
    .sanitize_environment()
    .env(env::DYLINT_BUILDING_LIBRARIES, "1")
    .current_dir(&tempdir)
    .success()
    .unwrap();
    let _: PathBuf =
        debug_path_with_toolchain(tempdir.path(), "fill_me_in", &msrv_toolchain).unwrap();

    patch_dylint_template(
        tempdir.path(),
        msrv::MSRV_PLUS_1_CHANNEL,
        msrv::MSRV_PLUS_1_CLIPPY_UTILS_REV,
    )
    .unwrap();
    let msrv_plus_1_toolchain = active_toolchain(tempdir.path()).unwrap();
    dylint_internal::cargo::build(&format!(
        "dylint-template with channel `{}`",
        msrv::MSRV_PLUS_1_CHANNEL
    ))
    .build()
    .sanitize_environment()
    .env(env::DYLINT_BUILDING_LIBRARIES, "1")
    .current_dir(&tempdir)
    .success()
    .unwrap();
    let _: PathBuf =
        debug_path_with_toolchain(tempdir.path(), "fill_me_in", &msrv_plus_1_toolchain).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        // smoelius: A toolchain can fail when it shares a target directory with another toolchain.
        // Setting `CARGO_INCREMENTAL` to 0 avoids this problem, but it is a band-aid. I need to
        // investigate ways of determining the toolchain used to produce a target directory.
        .env(env::CARGO_INCREMENTAL, "0")
        .env(
            env::DYLINT_LIBRARY_PATH,
            target_debug(tempdir.path()).unwrap(),
        )
        .args(["dylint", "list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!("fill_me_in@{msrv_toolchain}")).and(
                predicate::str::contains(format!("fill_me_in@{msrv_plus_1_toolchain}")),
            ),
        );
}

fn patch_dylint_template(path: &Path, channel: &str, clippy_utils_rev: &str) -> Result<()> {
    set_toolchain_channel(path, channel)?;
    set_clippy_utils_dependency_revision(path, clippy_utils_rev)?;
    Ok(())
}

// smoelius: This test will have to go away once `DYLINT_LIBRARY_PATH` is removed.
#[test]
fn one_name_multiple_paths() {
    let tempdirs = (tempdir().unwrap(), tempdir().unwrap());

    new_template(tempdirs.0.path()).unwrap();
    new_template(tempdirs.1.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdirs.0.path()))
        .build()
        .sanitize_environment()
        .env(env::DYLINT_BUILDING_LIBRARIES, "1")
        .current_dir(&tempdirs.0)
        .success()
        .unwrap();
    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdirs.1.path()))
        .build()
        .sanitize_environment()
        .env(env::DYLINT_BUILDING_LIBRARIES, "1")
        .current_dir(&tempdirs.1)
        .success()
        .unwrap();

    let _: PathBuf = debug_path_with_active_toolchain(tempdirs.0.path(), "fill_me_in").unwrap();
    let _: PathBuf = debug_path_with_active_toolchain(tempdirs.1.path(), "fill_me_in").unwrap();

    let paths = join_paths([
        &target_debug(tempdirs.0.path()).unwrap(),
        &target_debug(tempdirs.1.path()).unwrap(),
    ])
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .envs([(env::DYLINT_LIBRARY_PATH, paths)])
        .args(["dylint", "list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!(
                "fill_me_in ({})",
                target_debug(tempdirs.0.path()).unwrap().to_string_lossy()
            ))
            .and(predicate::str::contains(format!(
                "fill_me_in ({})",
                target_debug(tempdirs.1.path()).unwrap().to_string_lossy()
            ))),
        );
}

// smoelius: Once `DYLINT_LIBRARY_PATH` is removed, this test will need to be adjusted.
// Specifically, the checks for `fill_me_in` will need to go away.
#[test]
fn opts_library_package() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdir.path()))
        .build()
        .sanitize_environment()
        .env(env::DYLINT_BUILDING_LIBRARIES, "1")
        .current_dir(&tempdir)
        .success()
        .unwrap();

    let _: PathBuf = debug_path_with_active_toolchain(tempdir.path(), "fill_me_in").unwrap();

    let paths = join_paths([&target_debug(tempdir.path()).unwrap()]).unwrap();

    // smoelius: Sanity.
    cargo_bin_cmd!("cargo-dylint")
        .envs([(env::DYLINT_LIBRARY_PATH, &paths)])
        .args(["dylint", "list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fill_me_in"));

    cargo_bin_cmd!("cargo-dylint")
        .envs([(env::DYLINT_LIBRARY_PATH, paths)])
        .args([
            "dylint",
            "list",
            "--path",
            "../examples/general/crate_wide_allow",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("fill_me_in")
                .not()
                .and(predicate::str::contains("crate_wide_allow")),
        );
}

// smoelius: Once `DYLINT_LIBRARY_PATH` is removed, this test will need to go away or be adjusted.
// Re that latter, it might make sense to convert this test to one that uses `--path`.
#[test]
fn relative_path() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdir.path()))
        .build()
        .sanitize_environment()
        .env(env::DYLINT_BUILDING_LIBRARIES, "1")
        .current_dir(&tempdir)
        .success()
        .unwrap();

    let _: PathBuf = debug_path_with_active_toolchain(tempdir.path(), "fill_me_in").unwrap();

    for path in [
        tempdir.path().join("target/../target/debug"),
        tempdir.path().join("target/debug/../debug"),
    ] {
        let canonical_path = path.canonicalize().unwrap();

        assert_ne!(path, canonical_path);

        // smoelius: On Windows, `tempdir.path()` must be canonicalized to ensure it has a path
        // prefix. Otherwise, the call to `strip_prefix` could fail.
        let relative_path = canonical_path
            .strip_prefix(tempdir.path().canonicalize().unwrap())
            .unwrap();

        cargo_bin_cmd!("cargo-dylint")
            .current_dir(&tempdir)
            .envs([(env::DYLINT_LIBRARY_PATH, &path)])
            .args(["dylint", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains(relative_path.to_string_lossy()));
    }
}

// smoelius: This test checks that if a library path is passed to `--path`, a "No library packages
// found in ..." message is emitted.
#[test]
fn list_by_path() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdir.path()))
        .build()
        .sanitize_environment()
        .env(env::DYLINT_BUILDING_LIBRARIES, "1")
        .current_dir(&tempdir)
        .success()
        .unwrap();

    let path = debug_path_with_active_toolchain(tempdir.path(), "fill_me_in").unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .args(["dylint", "list", "--path", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No library packages found in "));
}

/// Returns the canonical path to the `target/debug` directory of the package at `dir`.
// smoelius: For the tests to pass on OSX, the paths have to be canonicalized, because `/var` is
// symlinked to `/private/var`.
fn target_debug(dir: &Path) -> Result<PathBuf> {
    let metadata = dylint_internal::cargo::metadata(dir)?;
    let debug_dir = metadata.target_directory.join("debug");
    debug_dir
        .canonicalize()
        .with_context(|| format!("Could not canonicalize {debug_dir:?}"))
}
