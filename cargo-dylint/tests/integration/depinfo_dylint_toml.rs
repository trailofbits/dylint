use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{CommandExt, env, packaging::isolate};
use predicates::prelude::*;
use std::{
    env::remove_var,
    fs::{OpenOptions, write},
    io::Write,
};
use tempfile::tempdir;

#[ctor::ctor(unsafe)]
fn initialize() {
    unsafe {
        remove_var(env::CARGO_TERM_COLOR);
    }
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

/// Verify that changes to `dylint.toml` cause the lints to be rerun.
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

    let dylint_toml = tempdir.path().join("dylint.toml");

    write(&dylint_toml, "[unnamed_constant]\nthreshold = 100\n").unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: unnamed constant").not());

    write(&dylint_toml, "[unnamed_constant]\nthreshold = 1\n").unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Checking depinfo_dylint_toml_test")
                .and(predicate::str::contains("warning: unnamed constant")),
        );
}
