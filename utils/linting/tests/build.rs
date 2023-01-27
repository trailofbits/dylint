use assert_cmd::{assert::AssertResult, prelude::*};
use dylint_internal::env;

#[test]
fn builds_with_cfg_docsrs() {
    update_nightly().unwrap();

    std::process::Command::new("cargo")
        .env(env::RUSTFLAGS, "--cfg docsrs")
        .args(["+nightly", "build"])
        .assert()
        .success();
}

#[test]
fn builds_with_latest_nightly() {
    update_nightly().unwrap();

    std::process::Command::new("cargo")
        .args(["+nightly", "build"])
        .assert()
        .success();
}

fn update_nightly() -> AssertResult {
    std::process::Command::new("rustup")
        .args(["update", "--no-self-update", "nightly"])
        .assert()
        .try_success()
}
