use assert_cmd::{assert::AssertResult, prelude::*};
use dylint_internal::env;
use std::sync::Mutex;

#[test]
fn builds_with_cfg_docsrs() {
    update_nightly().unwrap();

    std::process::Command::new("cargo")
        .env(env::RUSTFLAGS, "--cfg docsrs")
        .arg("build")
        .assert()
        .success();
}

#[test]
fn builds_with_latest_nightly() {
    update_nightly().unwrap();

    std::process::Command::new("cargo")
        .arg("build")
        .assert()
        .success();
}

// smoelius: Avoid: https://github.com/rust-lang/rustup/issues/988
static MUTEX: Mutex<()> = Mutex::new(());

fn update_nightly() -> AssertResult {
    let _lock = MUTEX.lock().unwrap();

    std::process::Command::new("rustup")
        .args(["update", "--no-self-update", "nightly"])
        .assert()
        .try_success()
}
