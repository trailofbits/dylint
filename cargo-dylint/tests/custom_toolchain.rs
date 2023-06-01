// smoelius: As per `dylint-link/src/main.rs`:
// "Only the MSVC toolchain is supported on Windows"
#![cfg(not(target_os = "windows"))]

use anyhow::{anyhow, Context, Result};
use dylint_internal::{
    clippy_utils::set_toolchain_channel,
    rustup::{toolchain_path, SanitizeEnvironment},
    testing::new_template,
    Command,
};
use std::path::Path;
use tempfile::{tempdir, NamedTempFile};

#[test]
fn custom_toolchain() {
    let tempdir = tempdir().unwrap();

    new_template(tempdir.path()).unwrap();

    let toolchain_path = toolchain_path(tempdir.path()).unwrap();

    let custom_toolchain = random_string().unwrap();

    link_toolchain(&custom_toolchain, &toolchain_path).unwrap();

    patch_dylint_template(tempdir.path(), &custom_toolchain).unwrap();

    dylint_internal::cargo::test(
        &format!("with custom toolchain `{custom_toolchain}`"),
        false,
    )
    .sanitize_environment()
    .current_dir(&tempdir)
    .success()
    .unwrap();

    uninstall_toolchain(&custom_toolchain).unwrap();
}

fn random_string() -> Result<String> {
    let tempfile = NamedTempFile::new().with_context(|| "Could not create named temp file")?;
    tempfile
        .path()
        .file_name()
        .map(|s| s.to_string_lossy().trim_start_matches('.').to_string())
        .ok_or_else(|| anyhow!("Could not get file name"))
}

fn link_toolchain(toolchain: &str, path: &Path) -> Result<()> {
    Command::new("rustup")
        .args(["toolchain", "link", toolchain, &path.to_string_lossy()])
        .success()
}

fn patch_dylint_template(path: &Path, channel: &str) -> Result<()> {
    set_toolchain_channel(path, channel)
}

fn uninstall_toolchain(toolchain: &str) -> Result<()> {
    Command::new("rustup")
        .args(["toolchain", "uninstall", toolchain])
        .success()
}
