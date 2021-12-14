// smoelius: On Windows, `rustup update nightly` generates "could not create link" errors similar to
// this one: https://github.com/rust-lang/rustup/issues/1316
#[cfg(not(target_os = "windows"))]
mod nightly_toolchain {
    use anyhow::Result;
    use dylint_internal::Command;
    use test_log::test;

    #[test]
    fn nightly_toolchain() {
        update_nightly().unwrap();

        let _ = dylint::driver_builder::get(&dylint::Dylint::default(), "nightly").unwrap();
    }

    fn update_nightly() -> Result<()> {
        Command::new("rustup")
            .args(&["update", "nightly"])
            .success()
    }
}
