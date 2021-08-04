use anyhow::Result;
use assert_cmd::prelude::*;
use dylint_internal::{env, rustup::SanitizeEnvironment, Command};
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

// smoelius: FIXME: Shell (Is it really a FIXME if I keep doing it?)
fn patch_dylint_template(path: &Path, channel: &str, clippy_utils_tag: &str) -> Result<()> {
    Command::new("sh")
        .current_dir(&path)
        .args(&[
            "-c",
            &format!(
                r#"
                    sed -i -e 's/^channel = "[^"]*"$/channel = "{}"/' rust-toolchain &&
                    sed -i -e 's/^\(clippy_utils\>.*\)\<tag = "[^"]*"/\1tag = "{}"/' Cargo.toml
                "#,
                channel, clippy_utils_tag,
            ),
        ])
        .success()
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
