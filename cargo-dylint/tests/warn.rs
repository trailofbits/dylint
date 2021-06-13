use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::path::Path;
use test_env_log::test;

#[test]
fn no_libraries_were_found() {
    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(Path::new("..").join("driver"))
        .args(&["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No libraries were found."));
}

#[test]
fn nothing_to_do() {
    dylint_internal::examples::build().unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&["dylint"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Nothing to do. Did you forget `--all`?",
        ));
}
