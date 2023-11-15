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
        .stderr(predicate::eq("Warning: No libraries were found.\n"));

    cargo_dylint()
        .args([
            "dylint",
            "list",
            "--manifest-path",
            &tempdir.path().join("Cargo.toml").to_string_lossy(),
        ])
        .assert()
        .success()
        .stderr(predicate::eq("Warning: No libraries were found.\n"));
}

#[test]
fn nothing_to_do() {
    cargo_dylint()
        .args(["dylint"])
        .assert()
        .success()
        .stderr(predicate::eq(
            "Warning: Nothing to do. Did you forget `--all`?\n",
        ));
}

// smoelius: If you build `cargo-dylint` directly (e.g., with `cargo run`), it gets built without
// the feature `dylint_internal/testing`, as you would expect. But if you build the integration
// tests (e.g., with `cargo test`), `cargo-dylint` gets built with that feature enabled. I don't
// understand why the difference.
//
// This problem was encountered in the `no_env_logger_warning` test as well.
fn cargo_dylint() -> std::process::Command {
    let mut command = std::process::Command::new("cargo");
    command.args(["run", "--quiet", "--bin", "cargo-dylint"]);
    command
}
