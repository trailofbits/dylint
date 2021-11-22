// smoelius: As per `dylint-link/src/main.rs`:
// "Only the MSVC toolchain is supported on Windows"
#[cfg(not(target_os = "windows"))]
mod custom_toolchain {
    use anyhow::{anyhow, Context, Result};
    use dylint_internal::{
        find_and_replace,
        rustup::{toolchain_path, SanitizeEnvironment},
        Command,
    };
    use std::path::Path;
    use tempfile::{tempdir, NamedTempFile};
    use test_log::test;

    #[test]
    fn custom_toolchain() {
        let tempdir = tempdir().unwrap();

        dylint_internal::clone_dylint_template(tempdir.path()).unwrap();

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
        let tempfile = NamedTempFile::new().with_context(|| "Could not create named temp file")?;
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
        // smoelius: See https://github.com/rust-lang/regex/issues/244
        find_and_replace(
            &path.join("rust-toolchain"),
            &[&format!(
                r#"s/(?m)^channel = "[^"]*"/channel = "{}"/"#,
                channel,
            )],
        )
    }

    fn uninstall_toolchain(toolchain: &str) -> Result<()> {
        Command::new("rustup")
            .args(&["toolchain", "uninstall", toolchain])
            .success()
    }
}
