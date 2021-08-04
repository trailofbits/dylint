use anyhow::{anyhow, Result};
use dylint_internal::{
    rustup::{toolchain_path, SanitizeEnvironment},
    Command,
};
use std::path::Path;
use tempfile::{tempdir, NamedTempFile};
use test_env_log::test;

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";

#[test]
fn custom_toolchain() {
    let tempdir = tempdir().unwrap();

    dylint_internal::checkout(DYLINT_TEMPLATE_URL, "master", tempdir.path()).unwrap();

    let toolchain_path = toolchain_path(tempdir.path()).unwrap();

    let custom_toolchain = random_string().unwrap();

    link_toolchain(&custom_toolchain, &toolchain_path).unwrap();

    patch_dylint_template(tempdir.path(), &custom_toolchain).unwrap();

    dylint_internal::test()
        .sanitize_environment()
        .current_dir(tempdir.path())
        .success()
        .unwrap();

    uninstall_toolchain(&custom_toolchain).unwrap();
}

fn random_string() -> Result<String> {
    let tempfile = NamedTempFile::new()?;
    tempfile
        .path()
        .file_name()
        .map(|s| s.to_string_lossy().trim_start_matches('.').to_string())
        .ok_or_else(|| anyhow!("Could not get file name"))
}

fn link_toolchain(toolchain: &str, path: &Path) -> Result<()> {
    Command::new("rustup")
        .args(&["toolchain", "link", toolchain, &path.to_string_lossy()])
        .success()
}

fn patch_dylint_template(path: &Path, channel: &str) -> Result<()> {
    Command::new("sh")
        .current_dir(&path)
        .args(&[
            "-c",
            &format!(
                r#"
                    sed -i -e 's/^channel = "[^"]*"$/channel = "{}"/' rust-toolchain
                "#,
                channel,
            ),
        ])
        .success()
}

fn uninstall_toolchain(toolchain: &str) -> Result<()> {
    Command::new("rustup")
        .args(&["toolchain", "uninstall", toolchain])
        .success()
}
