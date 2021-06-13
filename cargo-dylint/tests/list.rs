use anyhow::Result;
use assert_cmd::prelude::*;
use dylint_internal::{cargo::SanitizeEnvironment, env, Command};
use predicates::prelude::*;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use test_env_log::test;

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";
const DYLINT_TEMPLATE_REV: &str = "fc9d0e025d2d9d57ea9cc9bb1764047bbc8ae609";

const CHANNEL_A: &str = "nightly-2021-02-11";
const CHANNEL_B: &str = "nightly-2021-04-08";

const CLIPPY_UTILS_REV_A: &str = "454515040a580f72c9b6366ee7d46256cfb4246f";
const CLIPPY_UTILS_REV_B: &str = "586a99348c6a6f5309e82b340193067b7d76e37c";

#[test]
fn one_name_multiple_toolchains() {
    let tempdir = tempdir().unwrap();

    dylint_internal::checkout(DYLINT_TEMPLATE_URL, DYLINT_TEMPLATE_REV, tempdir.path()).unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_A, CLIPPY_UTILS_REV_A).unwrap();
    dylint_internal::build()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    patch_dylint_template(tempdir.path(), CHANNEL_B, CLIPPY_UTILS_REV_B).unwrap();
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
fn patch_dylint_template(path: &Path, channel: &str, clippy_utils_rev: &str) -> Result<()> {
    Command::new("sh")
        .current_dir(&path)
        .args(&[
            "-c",
            &format!(
                r#"
                    sed -i -e 's/^channel = "[^"]*"$/channel = "{}"/' rust-toolchain &&
                    sed -i -e 's/rev = "[^"]*"/rev = "{}"/' Cargo.toml
                "#,
                channel, clippy_utils_rev,
            ),
        ])
        .success()
}

#[test]
fn one_name_multiple_paths() {
    let tempdirs = (tempdir().unwrap(), tempdir().unwrap());

    dylint_internal::checkout(DYLINT_TEMPLATE_URL, DYLINT_TEMPLATE_REV, tempdirs.0.path()).unwrap();
    dylint_internal::checkout(DYLINT_TEMPLATE_URL, DYLINT_TEMPLATE_REV, tempdirs.1.path()).unwrap();

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

    // smoelius: https://users.rust-lang.org/t/osstring-osstr-error/35249
    let mut paths = OsString::new();
    paths.push(&target_debug(tempdirs.0.path()));
    paths.push(":");
    paths.push(&target_debug(tempdirs.1.path()));

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
