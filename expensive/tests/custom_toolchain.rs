#![cfg(not(coverage))]
// smoelius: As per `dylint-link/src/main.rs`:
// "Only the MSVC toolchain is supported on Windows"
#![cfg(not(target_os = "windows"))]

use anyhow::{anyhow, Context, Result};
use dylint_internal::{
    clippy_utils::set_toolchain_channel, find_and_replace, rustup::SanitizeEnvironment,
    testing::new_template, CommandExt,
};
use std::{path::Path, process::Command};
use tempfile::{tempdir, NamedTempFile, TempDir};

const RUST_URL: &str = "https://github.com/rust-lang/rust";

const TRIPLE: &str = if cfg!(target_os = "linux") {
    "x86_64-unknown-linux-gnu"
} else {
    "aarch64-apple-darwin"
};

#[test]
fn custom_toolchain() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    let (_toolchain_dir, custom_toolchain) = build_custom_toolchain().unwrap();

    patch_dylint_template(tempdir.path(), &custom_toolchain).unwrap();

    dylint_internal::cargo::test(&format!("with custom toolchain `{custom_toolchain}`"))
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    uninstall_toolchain(&custom_toolchain).unwrap();
}

fn build_custom_toolchain() -> Result<(TempDir, String)> {
    let tempdir = tempdir().unwrap();

    Command::new("git")
        .args([
            "clone",
            "--depth=1",
            RUST_URL,
            &tempdir.path().to_string_lossy(),
        ])
        .success()?;

    // smoelius: Build the stage 2 compiler. This makes the `rustc_private` crates available for the
    // stage 1 compiler.
    // smoelius: `.env_remove("GITHUB_ACTIONS")` is a hack to avoid building LLVM.
    Command::new("./x.py")
        .current_dir(&tempdir)
        .env_remove("GITHUB_ACTIONS")
        .args(["build", "--stage=2"])
        .success()?;

    let toolchain = random_string()?;

    // smoelius: Return a link to the stage 1 compiler.
    Command::new("rustup")
        .current_dir(&tempdir)
        .args([
            "toolchain",
            "link",
            &toolchain,
            &format!("build/{TRIPLE}/stage1"),
        ])
        .success()?;

    Ok((tempdir, toolchain))
}

fn random_string() -> Result<String> {
    let tempfile = NamedTempFile::new().with_context(|| "Could not create named temp file")?;
    tempfile
        .path()
        .file_name()
        .map(|s| s.to_string_lossy().trim_start_matches('.').to_string())
        .ok_or_else(|| anyhow!("Could not get file name"))
}

fn patch_dylint_template(path: &Path, channel: &str) -> Result<()> {
    // smoelius: `clippy_utils` may not build with the new toolchain.
    find_and_replace(&path.join("Cargo.toml"), "\r?\nclippy_utils = [^\r\n]*", "")?;

    set_toolchain_channel(path, channel)
}

fn uninstall_toolchain(toolchain: &str) -> Result<()> {
    Command::new("rustup")
        .args(["toolchain", "uninstall", toolchain])
        .success()
}
