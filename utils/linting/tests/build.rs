use assert_cmd::{assert::AssertResult, prelude::*};
use dylint_internal::env;
use rustc_version::{version_meta, Channel};
use std::sync::Mutex;
use tempfile::TempDir;

#[test]
fn channel_is_nightly() {
    assert!(matches!(version_meta().unwrap().channel, Channel::Nightly));
}

#[test]
fn builds_with_cfg_docsrs() {
    update_nightly().unwrap();

    let tempdir = TempDir::new().unwrap();

    std::process::Command::new("cargo")
        .env(env::RUSTFLAGS, "--cfg docsrs")
        .args(["build", "--target-dir", &tempdir.path().to_string_lossy()])
        .assert()
        .success();
}

// smoelius: Avoid: https://github.com/rust-lang/rustup/issues/988
static MUTEX: Mutex<()> = Mutex::new(());

#[allow(clippy::result_large_err)]
fn update_nightly() -> AssertResult {
    let _lock = MUTEX.lock().unwrap();

    std::process::Command::new("rustup")
        .args(["update", "--no-self-update", "nightly"])
        .assert()
        .try_success()
}
