use assert_cmd::prelude::*;
use dylint_internal::packaging::isolate;
use predicates::prelude::*;
use std::{fs::OpenOptions, io::Write};
use tempfile::{tempdir, tempdir_in};
use test_log::test;

// smoelius: "Separate lints into categories" commit
const REV: &str = "402fc24351c60a3c474e786fd76aa66aa8638d55";

#[test]
fn invalid_pattern() {
    for pattern in ["/*", "../*"] {
        let tempdir = tempdir().unwrap();

        dylint_internal::cargo::init("package `invalid_pattern_test`", false)
            .current_dir(&tempdir)
            .args(["--name", "invalid_pattern_test"])
            .success()
            .unwrap();

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(tempdir.path().join("Cargo.toml"))
            .unwrap();

        // smoelius: For the `../*` test to be effective, there must be multiple copies of Dylint in
        // Cargo's `checkouts` directory.
        write!(
            file,
            r#"
[workspace.metadata.dylint]
libraries = [
    {{ git = "https://github.com/trailofbits/dylint", pattern = "examples/general/crate_wide_allow", rev = "{REV}" }},
    {{ git = "https://github.com/trailofbits/dylint", pattern = "{pattern}" }},
]
"#,
        )
        .unwrap();

        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .current_dir(&tempdir)
            .args(["dylint", "--all"])
            .assert()
            .failure()
            .stderr(
                predicate::str::is_match(r#"Could not canonicalize "[^"]*""#)
                    .unwrap()
                    .or(predicate::str::is_match(
                        r#"Pattern `[^`]*` refers to paths outside of `[^`]*`"#,
                    )
                    .unwrap()),
            );
    }
}

#[test]
fn nonexistent_library() {
    let tempdir = tempdir_in(".").unwrap();

    dylint_internal::cargo::init("package `nonexistent_library_test`", false)
        .current_dir(&tempdir)
        .args(["--name", "nonexistent_library_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "../../examples/general/crate_wide_allow"
"#
    )
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "../../examples/general/nonexistent_library"
"#
    )
    .unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No paths matched"));
}
