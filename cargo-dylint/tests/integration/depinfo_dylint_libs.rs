use assert_cmd::prelude::*;
use dylint_internal::env;
use predicates::prelude::*;
use std::{env::set_var, process::Command};

#[ctor::ctor]
fn initialize() {
    unsafe {
        set_var(env::CARGO_TERM_COLOR, "never");
    }
}

#[test]
fn depinfo_dylint_libs() {
    Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir("../fixtures/depinfo_dylint_libs")
        .args(["dylint", "--lib", "question_mark_in_expression"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: using the `?` operator within an expression\n",
        ));

    Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir("../fixtures/depinfo_dylint_libs")
        .args(["dylint", "--lib", "try_io_result"])
        .assert()
        .stderr(predicate::str::contains(
            "\nwarning: returning a `std::io::Result` could discard relevant context (e.g., files \
             or paths involved)\n",
        ));
}
