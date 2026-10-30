use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{env, testing::copy_dir};
use predicates::prelude::*;
use std::path::Path;
use tempfile::tempdir_in;

#[test]
fn depinfo_dylint_libs() {
    let temp_fixture = tempdir_in("../fixtures").unwrap();
    copy_dir(Path::new("../fixtures/depinfo"), temp_fixture.path()).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .args(["dylint", "--lib", "question_mark_in_expression"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: using the `?` operator within an expression\n",
        ));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .args(["dylint", "--lib", "try_io_result"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: returning a `std::io::Result` could discard relevant context (e.g., files \
             or paths involved)\n",
        ));
}

#[test]
fn depinfo_dylint_rustflags() {
    let temp_fixture = tempdir_in("../fixtures").unwrap();
    copy_dir(Path::new("../fixtures/depinfo"), temp_fixture.path()).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .args(["dylint", "--lib", "question_mark_in_expression"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: using the `?` operator within an expression\n",
        ));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .env(env::DYLINT_RUSTFLAGS, "-D warnings")
        .args(["dylint", "--lib", "question_mark_in_expression"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "\nerror: using the `?` operator within an expression\n",
        ));
}
