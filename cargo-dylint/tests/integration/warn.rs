use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn no_libraries_were_found() {
    let tempdir = tempdir().unwrap();

    std::process::Command::new("cargo")
        .current_dir(&tempdir)
        .args([
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

    cargo_dylint()
        .args([
            "dylint",
            "--all",
            "--manifest-path",
            &tempdir.path().join("Cargo.toml").to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: No libraries were found.\n",
        ));

    cargo_dylint()
        .args([
            "dylint",
            "list",
            "--manifest-path",
            &tempdir.path().join("Cargo.toml").to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: No libraries were found.\n",
        ));
}

#[test]
fn nothing_to_do() {
    cargo_dylint()
        .args(["dylint"])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: Nothing to do. Did you forget `--all`?\n",
        ));
}

/// `--all` should not be required when `--git` or `--path` is used on the command line.
#[test]
fn opts_library_package_no_warn() {
    let tempdir = tempdir().unwrap();

    std::process::Command::new("cargo")
        .current_dir(&tempdir)
        .args([
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

    cargo_dylint()
        .args([
            "dylint",
            "--manifest-path",
            &tempdir.path().join("Cargo.toml").to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: Nothing to do. Did you forget `--all`?\n",
        ));

    cargo_dylint()
        .args([
            "dylint",
            "--manifest-path",
            &tempdir.path().join("Cargo.toml").to_string_lossy(),
            "--path",
            "../examples/general/crate_wide_allow",
        ])
        .assert()
        .success()
        .stderr(
            predicate::str::ends_with("Warning: Nothing to do. Did you forget `--all`?\n").not(),
        );
}

// smoelius: If you build `cargo-dylint` directly (e.g., with `cargo run`), it gets built without
// the feature `dylint_internal/testing`, as you would expect. But if you build the integration
// tests (e.g., with `cargo test`), `cargo-dylint` gets built with that feature enabled. I don't
// understand why the difference.
//
// This problem was encountered in the `no_env_logger_warning` test as well.
#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
fn cargo_dylint() -> std::process::Command {
    /* let mut command = std::process::Command::new("cargo");
    command.args(["run", "--quiet", "--bin", "cargo-dylint"]);
    command */
    std::process::Command::cargo_bin("cargo-dylint").unwrap()
}
