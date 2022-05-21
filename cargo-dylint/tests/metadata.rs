use assert_cmd::prelude::*;
use dylint_internal::packaging::{isolate, use_local_packages};
use predicates::prelude::*;
use std::{fs::OpenOptions, io::Write};
use tempfile::tempdir_in;
use test_log::test;

#[test]
fn nonexistent_library() {
    let tempdir = tempdir_in(".").unwrap();

    dylint_internal::cargo::init(&format!("package `nonexistent_library_test`"), false)
        .current_dir(tempdir.path())
        .args(&["--name", "nonexistent_library_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();
    use_local_packages(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "../../examples/crate_wide_allow"
"#
    )
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(tempdir.path())
        .args(&["dylint", "--all"])
        .assert()
        .success();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "../../examples/nonexistent_library"
"#
    )
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(tempdir.path())
        .args(&["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No paths matched"));
}
