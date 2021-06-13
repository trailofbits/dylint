use assert_cmd::prelude::*;
use dylint_internal::env;
use predicates::prelude::*;
use std::path::Path;
use test_env_log::test;

#[test]
fn no_libraries_were_found() {
    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .env_remove(env::DYLINT_LIBRARY_PATH)
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
        .envs(vec![(
            env::DYLINT_LIBRARY_PATH,
            Path::new("..")
                .join("examples")
                .join("allow_clippy")
                .join("target")
                .join("debug")
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        )])
        .args(&["dylint"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Nothing to do. Did you forget `--all`?",
        ));
}
