use anyhow::Result;
use assert_cmd::prelude::*;
use dylint_internal::{env, find_and_replace, rustup::SanitizeEnvironment};
use predicates::prelude::*;
use std::{
    env::join_paths,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use test_env_log::test;

const CHANNEL_A: &str = "nightly-2021-03-11";
const CHANNEL_B: &str = "nightly-2021-04-22";

const CLIPPY_UTILS_TAG_A: &str = "rust-1.52.1";
const CLIPPY_UTILS_TAG_B: &str = "rust-1.53.0";

#[test]
fn one_name_multiple_toolchains() {
    let tempdir = tempdir().unwrap();

    dylint_internal::checkout_dylint_template(tempdir.path()).unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_A, CLIPPY_UTILS_TAG_A).unwrap();
    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_B, CLIPPY_UTILS_TAG_B).unwrap();
    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .envs(vec![(
            env::DYLINT_LIBRARY_PATH,
            target_debug(tempdir.path()),
        )])
        .args(&["dylint", "--list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(&format!("fill_me_in@{}", CHANNEL_A)).and(
                predicate::str::contains(&format!("fill_me_in@{}", CHANNEL_B)),
            ),
        );
}

fn patch_dylint_template(path: &Path, channel: &str, clippy_utils_tag: &str) -> Result<()> {
    find_and_replace(
        &path.join("rust-toolchain"),
        &[&format!(
            r#"s/(?m)^channel = "[^"]*"$/channel = "{}"/"#,
            channel,
        )],
    )?;
    find_and_replace(
        &path.join("Cargo.toml"),
        &[&format!(
            r#"s/(?m)^(clippy_utils\b.*)\btag = "[^"]*"/${{1}}tag = "{}"/"#,
            clippy_utils_tag,
        )],
    )
}

#[test]
fn one_name_multiple_paths() {
    let tempdirs = (tempdir().unwrap(), tempdir().unwrap());

    dylint_internal::checkout_dylint_template(tempdirs.0.path()).unwrap();
    dylint_internal::checkout_dylint_template(tempdirs.1.path()).unwrap();

    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdirs.0.path())
        .success()
        .unwrap();

    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdirs.1.path())
        .success()
        .unwrap();

    let paths = join_paths(&[
        &target_debug(tempdirs.0.path()),
        &target_debug(tempdirs.1.path()),
    ])
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .envs(vec![(env::DYLINT_LIBRARY_PATH, paths)])
        .args(&["dylint", "--list", "--all", "--no-metadata"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(&format!(
                "fill_me_in ({})",
                target_debug(tempdirs.0.path()).to_string_lossy()
            ))
            .and(predicate::str::contains(&format!(
                "fill_me_in ({})",
                target_debug(tempdirs.1.path()).to_string_lossy()
            ))),
        );
}

// smoelius: For the tests to pass on OSX, the paths have to be canonicalized, because `/var` is
// symlinked to `/private/var`.
fn target_debug(path: &Path) -> PathBuf {
    path.canonicalize().unwrap().join("target").join("debug")
}
