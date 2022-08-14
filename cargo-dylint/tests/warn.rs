use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::tempdir;
use test_log::test;

#[test]
fn no_libraries_were_found() {
    let tempdir = tempdir().unwrap();

    std::process::Command::new("cargo")
        .current_dir(tempdir.path())
        .args(&[
            "init",
            "--name",
            tempdir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches('.'),
        ])
        .assert()
        .success();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(tempdir.path())
        .args(&["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No libraries were found."));

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(tempdir.path())
        .args(&["dylint", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No libraries were found."));
}

#[test]
fn nothing_to_do() {
    // smoelius: The code that handles workspace metadata builds the example libraries, so
    // `examples::build()` is no longer needed here.
    // dylint_internal::examples::build().unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .args(&["dylint"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Nothing to do. Did you forget `--all`?")
                .and(predicate::str::contains("Building").not()),
        );
}
