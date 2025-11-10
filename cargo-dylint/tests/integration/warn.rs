use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn no_libraries_were_found() {
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: No libraries were found.\n",
        ));

    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "list"])
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
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint"])
        .assert()
        .success()
        .stderr(predicate::str::ends_with(
            "Warning: Nothing to do. Did you forget `--all`?\n",
        ));

    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args([
            "dylint",
            "--path",
            "../../examples/general/crate_wide_allow",
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
#[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
fn cargo_dylint() -> assert_cmd::Command {
    /* let mut command = std::process::Command::new("cargo");
    command.args(["run", "--quiet", "--bin", "cargo-dylint"]);
    command */
    cargo_bin_cmd!("cargo-dylint")
}
