use assert_cmd::{assert::AssertResult, prelude::*};

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
