use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{CommandExt, env, packaging::isolate, testing::copy_dir};
use predicates::prelude::*;
use std::{
    fs::{OpenOptions, remove_file, write},
    io::Write,
    path::Path,
};
use tempfile::{tempdir, tempdir_in};

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

// `unnamed_constant` flags `9` when its `threshold` is 1, but not when its `threshold` is 100.
// Note that `9` is also unflagged at the default `threshold` of 10. Hence, the warning below can
// result only from `dylint.toml` having been reread.
const MAIN_RS: &str = "\
fn main() {
    let mut x: u64 = 1;
    x *= 9;
    println!(\"{x}\");
}
";

/// Verify that creating, changing, and removing `dylint.toml` rerun lints only when necessary.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn depinfo_dylint_toml() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `depinfo_dylint_toml_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "depinfo_dylint_toml_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "{}/../examples/supplementary/unnamed_constant"
"#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    write(tempdir.path().join("src/main.rs"), MAIN_RS).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Checking depinfo_dylint_toml_test")
                .and(predicate::str::contains("warning: unnamed constant").not()),
        );

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Fresh depinfo_dylint_toml_test")
                .and(predicate::str::contains("Checking depinfo_dylint_toml_test").not()),
        );

    let dylint_toml = tempdir.path().join("dylint.toml");

    write(&dylint_toml, "[unnamed_constant]\nthreshold = 100\n").unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Checking depinfo_dylint_toml_test")
                .and(predicate::str::contains("warning: unnamed constant").not()),
        );

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Fresh depinfo_dylint_toml_test")
                .and(predicate::str::contains("Checking depinfo_dylint_toml_test").not()),
        );

    write(&dylint_toml, "[unnamed_constant]\nthreshold = 1\n").unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Checking depinfo_dylint_toml_test")
                .and(predicate::str::contains("warning: unnamed constant")),
        );

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Fresh depinfo_dylint_toml_test")
                .and(predicate::str::contains("Checking depinfo_dylint_toml_test").not()),
        );

    remove_file(&dylint_toml).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all", "--", "--verbose"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Checking depinfo_dylint_toml_test")
                .and(predicate::str::contains("warning: unnamed constant").not()),
        );
}
