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
fn fail_on_no_libraries() {
    // `--all` selects libraries but none are found, so the check fails. The message appears as an
    // error only, not also as a warning.
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "--all", "--fail-on-no-libraries"])
        .assert()
        .failure()
        .stderr(
            predicate::str::ends_with("Error: No libraries were found\n")
                .and(predicate::str::contains("Warning: No libraries were found").not()),
        );

    // `list` fails when libraries are selected but none are found.
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "list", "--all", "--fail-on-no-libraries"])
        .assert()
        .failure()
        .stderr(
            predicate::str::ends_with("Error: No libraries were found\n")
                .and(predicate::str::contains("Warning: No libraries were found").not()),
        );

    // `--fail-on-no-libraries` without a library selection is an error. `run` reports it before
    // reaching the code that would otherwise warn about having nothing to do.
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "--fail-on-no-libraries"])
        .assert()
        .failure()
        .stderr(
            predicate::str::ends_with(
                "Error: `--fail-on-no-libraries` requires `--all`, `--git`, `--lib`, \
                 `--lib-path`, or `--path`\n",
            )
            .and(predicate::str::contains("Nothing to do").not()),
        );

    // `list` can run without a library selection, but not with `--fail-on-no-libraries`.
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args(["dylint", "list", "--fail-on-no-libraries"])
        .assert()
        .failure()
        .stderr(
            predicate::str::ends_with(
                "Error: `--fail-on-no-libraries` requires `--all`, `--git`, `--lib`, \
                 `--lib-path`, or `--path`\n",
            )
            .and(predicate::str::contains("Nothing to do").not()),
        );

    // `--path` counts as a library selection because `run` sets `all` when `--git` or `--path` is
    // used. The check then runs to completion.
    cargo_dylint()
        .current_dir("../fixtures/empty")
        .args([
            "dylint",
            "--path",
            "../../examples/general/crate_wide_allow",
            "--fail-on-no-libraries",
        ])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("crate_wide_allow")
                .and(predicate::str::contains("Checking with toolchain")),
        );

    // With a library available, `--fail-on-no-libraries` has no effect: the lints are listed and
    // the command succeeds.
    cargo_dylint()
        .current_dir("../fixtures/no_deps")
        .args(["dylint", "list", "--all", "--fail-on-no-libraries"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "`?` operators embedded within an expression",
        ))
        .stderr(predicate::str::contains("No libraries were found").not());
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
