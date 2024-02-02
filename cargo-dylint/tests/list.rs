// smoelius: As of version 0.1.14, `cargo-llvm-cov` no longer sets `CARGO_TARGET_DIR`. So it is now
// safe to run these tests under `cargo-llvm-cov`.
// #![cfg(not(coverage))]

use anyhow::{Context, Result};
use assert_cmd::prelude::*;
use cargo_metadata::MetadataCommand;
use dylint_internal::{
    clippy_utils::{set_clippy_utils_dependency_revision, set_toolchain_channel},
    env, library_filename,
    rustup::SanitizeEnvironment,
    testing::new_template,
    CommandExt,
};
use glob::glob;
use predicates::prelude::*;
use std::{
    env::join_paths,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

const CHANNEL_A: &str = "nightly-2023-06-29";
const CHANNEL_B: &str = "nightly-2023-07-14";

const CLIPPY_UTILS_REV_A: &str = "dd8e44c5a22ab646821252604420c5bb82c36aa9";
const CLIPPY_UTILS_REV_B: &str = "1d334696587ac22b3a9e651e7ac684ac9e0697b2";

#[test]
fn one_name_multiple_toolchains() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_A, CLIPPY_UTILS_REV_A).unwrap();
    dylint_internal::cargo::build(&format!("dylint-template with channel `{CHANNEL_A}`"))
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_B, CLIPPY_UTILS_REV_B).unwrap();
    dylint_internal::cargo::build(&format!("dylint-template with channel `{CHANNEL_B}`"))
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .envs([(
            env::DYLINT_LIBRARY_PATH,
            target_debug(tempdir.path()).unwrap(),
        )])
        .args(["dylint", "list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!("fill_me_in@{CHANNEL_A}"))
                .and(predicate::str::contains(format!("fill_me_in@{CHANNEL_B}"))),
        );
}

fn patch_dylint_template(path: &Path, channel: &str, clippy_utils_rev: &str) -> Result<()> {
    set_toolchain_channel(path, channel)?;
    set_clippy_utils_dependency_revision(path, clippy_utils_rev)?;
    Ok(())
}

#[test]
fn one_name_multiple_paths() {
    let tempdirs = (tempdir().unwrap(), tempdir().unwrap());

    new_template(tempdirs.0.path()).unwrap();
    new_template(tempdirs.1.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdirs.0.path()))
        .build()
        .sanitize_environment()
        .current_dir(&tempdirs.0)
        .success()
        .unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdirs.1.path()))
        .build()
        .sanitize_environment()
        .current_dir(&tempdirs.1)
        .success()
        .unwrap();

    let paths = join_paths([
        &target_debug(tempdirs.0.path()).unwrap(),
        &target_debug(tempdirs.1.path()).unwrap(),
    ])
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
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

#[test]
fn relative_path() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdir.path()))
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

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

        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .current_dir(&tempdir)
            .envs([(env::DYLINT_LIBRARY_PATH, &path)])
            .args(["dylint", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains(relative_path.to_string_lossy()));
    }
}

#[test]
fn list_by_path() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    dylint_internal::cargo::build(&format!("dylint-template in {:?}", tempdir.path()))
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    let path = glob(
        &tempdir
            .path()
            .join("target/debug")
            .join(library_filename("fill_me_in", "*"))
            .to_string_lossy(),
    )
    .ok()
    .as_mut()
    .and_then(Iterator::next)
    .unwrap()
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(["dylint", "list", "--path", &path.to_string_lossy()])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("fill_me_in").and(predicate::str::contains("Building").not()),
        )
        .stderr(predicate::str::contains(
            "Referring to libraries with `--path` is deprecated. Use `--lib-path`.",
        ));
}

// smoelius: For the tests to pass on OSX, the paths have to be canonicalized, because `/var` is
// symlinked to `/private/var`.
fn target_debug(path: &Path) -> Result<PathBuf> {
    let metadata = MetadataCommand::new().current_dir(path).no_deps().exec()?;
    let debug_dir = metadata.target_directory.join("debug");
    debug_dir
        .canonicalize()
        .with_context(|| format!("Could not canonicalize {debug_dir:?}"))
        .map_err(Into::into)
}
