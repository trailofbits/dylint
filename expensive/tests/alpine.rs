#![cfg(all(not(coverage), unix))]

use std::{
    io::{Write, stderr},
    process::Command,
};

#[test]
fn alpine() {
    let status = Command::new("which").arg("docker").status().unwrap();
    if !status.success() {
        #[allow(clippy::explicit_write)]
        writeln!(
            stderr(),
            "Skipping `alpine` test as `docker` could not be found",
        )
        .unwrap();
        return;
    }

    // smoelius: Don't use `assert_cmd::Command` here because it would hide the output.
    let status = Command::new("docker")
        .args([
            "build",
            "--progress=plain",
            "-f",
            "expensive/tests/Dockerfile",
            ".",
        ])
        .current_dir("..")
        .status()
        .unwrap();
    assert!(status.success());

    Command::new("docker")
        .args(["system", "prune", "--force"])
        .current_dir("..")
        .status()
        .unwrap();
    assert!(status.success());
}
