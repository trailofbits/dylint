use assert_cmd::{cargo::cargo_bin_cmd, prelude::*};
use dylint_internal::{CommandExt, env, packaging::isolate};
use predicates::prelude::*;
use std::{env::remove_var, fs::OpenOptions, io::Write};
use tempfile::tempdir;

// smoelius: "Separate lints into categories" commit
const REV: &str = "402fc24351c60a3c474e786fd76aa66aa8638d55";

#[ctor::ctor]
fn initialize() {
    unsafe {
        remove_var(env::CARGO_TERM_COLOR);
    }
}

#[test]
fn array_pattern() {
    let assert = cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/array_pattern")
        .args(["dylint", "list"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(2, lines.len());
    assert!(lines[0].starts_with("clippy "));
    assert!(lines[1].starts_with("question_mark_in_expression "));
}

#[cfg(unix)]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn edition_2021() {
    use dylint_internal::rustup::SanitizeEnvironment;
    use std::{fs::create_dir, os::unix::fs::symlink, path::Path};

    let cargo_home = tempdir().unwrap();
    let cargo_home_bin = cargo_home.path().join("bin");
    create_dir(&cargo_home_bin).unwrap();
    let cargo_dylint = env!("CARGO_BIN_EXE_cargo-dylint");
    symlink(cargo_dylint, cargo_home_bin.join("cargo-dylint")).unwrap();

    // smoelius: Sanity. Because this test is likely to have been run by `cargo test`, the
    // environment variables `CARGO`, etc. will have already been set. Thus, we must call
    // `sanitize_environment` to clear them.
    std::process::Command::new("rustup")
        .sanitize_environment()
        .current_dir("../fixtures/edition_2021")
        .args(["which", "cargo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1.84"));

    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/general/crate_wide_allow"
    ));

    // smoelius: The next command must be `cargo` so that we invoke the `rustup` proxy. The command
    // cannot be `cargo-dylint`.
    std::process::Command::new("cargo")
        .sanitize_environment()
        .current_dir("../fixtures/edition_2021")
        .env("CARGO_HOME", cargo_home.path())
        .args(["dylint", "--path", &path.to_string_lossy()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("2024").not());
}

#[test]
fn invalid_pattern() {
    for pattern in ["/*", "../*"] {
        let tempdir = tempdir().unwrap();

        dylint_internal::cargo::init("package `invalid_pattern_test`")
            .build()
            .current_dir(&tempdir)
            .args(["--name", "invalid_pattern_test"])
            .success()
            .unwrap();

        let mut file = OpenOptions::new()
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

        cargo_bin_cmd!("cargo-dylint")
            .current_dir(&tempdir)
            .args(["dylint", "--all"])
            .assert()
            .failure()
            .stderr(
                predicate::str::is_match(r#"Could not canonicalize "[^"]*""#)
                    .unwrap()
                    .or(predicate::str::is_match(
                        r"Pattern `[^`]*` could refer to `[^`]*`, which is outside of `[^`]*`",
                    )
                    .unwrap()),
            );
    }
}

#[test]
fn library_packages_in_dylint_toml() {
    cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/library_packages_in_dylint_toml")
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "\nwarning: `unwrap`s that could be combined\n",
        ));
}

#[test]
fn library_packages_with_rust_toolchain() {
    let assert = cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/library_packages_with_rust_toolchain")
        .env(env::RUST_LOG, "debug")
        .args(["dylint", "--all"])
        .assert()
        .success();

    if cfg!(all(
        feature = "cargo-cli",
        target_arch = "x86_64",
        not(target_os = "windows")
    )) {
        assert.stderr(predicate::str::contains(
            r#"/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo" "fetch""#,
        ));
    }
}

#[test]
fn list() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `list_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "list_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#,
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<unbuilt>"));
}

/// Verify that changes to workspace metadata cause the lints to be rerun.
#[test]
fn metadata_change() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `metadata_change_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "metadata_change_test"])
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
    path = "{}/../examples/general/crate_wide_allow"
    "#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test"));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test").not());

    write!(
        file,
        r#"
        [[workspace.metadata.dylint.libraries]]
        path = "{}/../examples/restriction/question_mark_in_expression"
        "#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test"));
}

#[test]
fn nonexistent_git_library() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `nonexistent_git_library_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "nonexistent_git_library_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/nonexistent_library"
"#
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No paths matched"));
}

#[test]
fn nonexistent_path_library() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `nonexistent_path_library_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "nonexistent_path_library_test"])
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
path = "{}/../examples/general/crate_wide_allow"
"#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
path = "{}/../examples/general/nonexistent_library"
"#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No library packages found in"));
}

/// Verify that changes to `RUSTFLAGS` do not cause workspace metadata entries to be rebuilt.
#[test]
fn rustflags_change() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `rustflags_change_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "rustflags_change_test"])
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
path = "{}/../examples/general/crate_wide_allow"
"#,
        env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\")
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Compiling"));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .env(env::RUSTFLAGS, "--verbose")
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Compiling").not());
}

#[test]
fn unknown_keys() {
    let tempdir = tempdir().unwrap();

    dylint_internal::cargo::init("package `unknown_keys_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "unknown_keys_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#,
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    writeln!(file, r#"revision = "{REV}""#,).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"Unknown library keys:\r?\n\s*revision\r?\n").unwrap());
}
