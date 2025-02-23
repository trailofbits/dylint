use dylint_internal::{
    CommandExt, driver as dylint_driver, env,
    rustup::{SanitizeEnvironment, toolchain_path},
    testing::new_template,
};
use std::fs::create_dir_all;
use tempfile::tempdir_in;

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn dylint_driver_path() {
    let tempdir = tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();

    new_template(tempdir.path()).unwrap();

    let dylint_driver_path = tempdir.path().join("target/dylint_drivers");

    create_dir_all(&dylint_driver_path).unwrap();

    dylint_internal::cargo::test("dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .envs([(env::DYLINT_DRIVER_PATH, &*dylint_driver_path)])
        .success()
        .unwrap();

    // smoelius: Verify that the driver can be run directly.
    // https://github.com/trailofbits/dylint/issues/54
    let toolchain_path = toolchain_path(tempdir.path()).unwrap();
    let toolchain = toolchain_path.iter().next_back().unwrap();
    let mut command = dylint_driver(
        &toolchain.to_string_lossy(),
        &dylint_driver_path.join(toolchain).join("dylint-driver"),
    )
    .unwrap();
    command.success().unwrap();
}
