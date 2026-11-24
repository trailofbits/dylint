use assert_cmd::cargo::cargo_bin_cmd;
use std::{
    fs::{create_dir_all, read_to_string, write},
    process::Command,
};
use tempfile::tempdir;

#[test]
fn locked_and_frozen_do_not_rewrite_lockfile() {
    let tempdir = tempdir().unwrap();
    let ws = tempdir.path();

    write(
        ws.join("Cargo.toml"),
        r#"[workspace]
members = ["a", "b"]
resolver = "3"
"#,
    )
    .unwrap();

    let a_dir = ws.join("a");
    let b_dir = ws.join("b");
    create_dir_all(a_dir.join("src")).unwrap();
    create_dir_all(b_dir.join("src")).unwrap();

    write(
        a_dir.join("Cargo.toml"),
        r#"[package]
name = "a"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    write(a_dir.join("src").join("lib.rs"), "").unwrap();

    write(
        b_dir.join("Cargo.toml"),
        r#"[package]
name = "b"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    write(b_dir.join("src").join("lib.rs"), "").unwrap();

    let status = Command::new("cargo")
        .args(["generate-lockfile", "--quiet"])
        .current_dir(ws)
        .status()
        .unwrap();
    assert!(status.success());

    let lock_path = ws.join("Cargo.lock");
    let orig_lock = read_to_string(&lock_path).unwrap();

    // lyn1dev: Add dependency to `a` without updating the lockfile.
    write(
        a_dir.join("Cargo.toml"),
        r#"[package]
name = "a"
version = "0.1.0"
edition = "2024"

[dependencies]
b = { path = "../b" }
"#,
    )
    .unwrap();

    for flag in ["--locked", "--frozen"] {
        write(&lock_path, &orig_lock).unwrap();

        cargo_bin_cmd!("cargo-dylint")
            .current_dir(ws)
            .args(["dylint", "--all", "--", flag])
            .assert()
            .success();

        let current_lock = read_to_string(&lock_path).unwrap();
        assert_eq!(orig_lock, current_lock);
    }
}
