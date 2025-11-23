use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::env;
use predicates::prelude::*;
use std::env::remove_var;

#[ctor::ctor]
fn initialize() {
    unsafe {
        remove_var(env::CARGO_TERM_COLOR);
    }
}

#[test]
fn depinfo_dylint_libs() {
    cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/depinfo_dylint_libs")
        .args(["dylint", "--lib", "question_mark_in_expression"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: using the `?` operator within an expression\n",
        ));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/depinfo_dylint_libs")
        .args(["dylint", "--lib", "try_io_result"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: returning a `std::io::Result` could discard relevant context (e.g., files \
             or paths involved)\n",
        ));
}
