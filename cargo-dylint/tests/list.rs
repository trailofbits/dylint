use anyhow::Result;
use assert_cmd::prelude::*;
use dylint_internal::{cargo::SanitizeEnvironment, env, Command};
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use test_env_log::test;

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";
const DYLINT_TEMPLATE_REV: &str = "43fec254e0e3cae1ba3e2e483585e333539ad192";

const CHANNEL_A: &str = "nightly-2021-02-11";
const CHANNEL_B: &str = "nightly-2021-03-11";

const CLIPPY_UTILS_REV_A: &str = "454515040a580f72c9b6366ee7d46256cfb4246f";
const CLIPPY_UTILS_REV_B: &str = "1a206fc4abae0b57a3f393481367cf3efca23586";

#[test]
fn one_name_mutltiple_toolchains() {
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
        .current_dir(tempdir.path())
        .env_remove(env::DYLINT_LIBRARY_PATH)
        .args(&["dylint", "--list", "--all"])
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
fn one_name_mutltiple_paths() {
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

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(tempdirs.0.path())
        .envs(vec![(
            env::DYLINT_LIBRARY_PATH,
            target_debug(tempdirs.1.path()),
        )])
        .args(&["dylint", "--list", "--all"])
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

fn target_debug(path: &Path) -> PathBuf {
    path.join("target").join("debug")
}
