use dylint_internal::{
    CommandExt, cargo, driver as dylint_driver, env,
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

    let dylint_home = tempdir.path().join("target/dylint_home");

    #[allow(deprecated)]
    let output = cargo::test("dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .envs([
            (env::DYLINT_DRIVER_PATH, &*dylint_driver_path),
            (env::DYLINT_HOME, &*dylint_home),
        ])
        .logged_output(true)
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("`DYLINT_DRIVER_PATH` is deprecated"),
        "{output:?}"
    );

    // `DYLINT_DRIVER_PATH` takes precedence over `DYLINT_HOME`.
    assert!(!dylint_home.exists());

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

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn dylint_home() {
    let tempdir = tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();

    new_template(tempdir.path()).unwrap();

    // `DYLINT_HOME` need not exist beforehand.
    let dylint_home = tempdir.path().join("target/dylint home");

    #[allow(deprecated)]
    cargo::test("dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .env_remove(env::DYLINT_DRIVER_PATH)
        .env(env::DYLINT_HOME, &dylint_home)
        .success()
        .unwrap();

    let toolchain_path = toolchain_path(tempdir.path()).unwrap();
    let toolchain = toolchain_path.iter().next_back().unwrap();
    let drivers = dylint_home.join("drivers");
    assert!(drivers.join("README.txt").is_file());
    let mut command = dylint_driver(
        &toolchain.to_string_lossy(),
        &drivers.join(toolchain).join("dylint-driver"),
    )
    .unwrap();
    command.success().unwrap();
}
