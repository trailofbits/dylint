use super::dylint_with_home;
use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{
    CommandExt, driver as dylint_driver, env,
    rustup::{SanitizeEnvironment, active_toolchain, toolchain_path},
    testing::{copy_dir, new_template},
};
use predicates::prelude::*;
use std::{
    fs::{create_dir_all, read_dir, rename},
    path::Path,
};
use tempfile::{tempdir, tempdir_in};

#[allow(deprecated)]
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

#[test]
#[allow(deprecated)]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
fn cache_dir_relocates_artifacts() {
    let home = tempdir().unwrap();
    let fixture = tempdir_in("../fixtures").unwrap();
    copy_dir(
        Path::new("../fixtures/library_packages_in_dylint_toml"),
        fixture.path(),
    )
    .unwrap();
    let root = tempdir().unwrap();
    let cache = root.path().join("custom cache");
    let command = |cache: &Path| {
        let mut command = dylint_with_home(home.path());
        command
            .current_dir(&fixture)
            .env(env::DYLINT_CACHE_DIR, cache)
            .env_remove(env::DYLINT_DRIVER_PATH)
            .env(env::CARGO_TARGET_DIR, fixture.path().join("target"))
            .env_remove(env::CARGO_BUILD_BUILD_DIR)
            .args(["dylint", "--all"]);
        command
    };

    command(&cache)
        .assert()
        .success()
        .stderr(predicate::str::contains("DYLINT_DRIVER_PATH").not());
    assert!(!home.path().join(".dylint").exists());
    let drivers = read_dir(cache.join("dylint-driver"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .unwrap();
    let toolchain =
        active_toolchain(Path::new("../examples/restriction/collapsible_unwrap")).unwrap();
    assert!(drivers.join(toolchain).join("dylint-driver").is_file());
    assert!(cache.join("collapsible_unwrap").is_dir());

    let other_cache = root.path().join("another cache");
    create_dir_all(&other_cache).unwrap();
    rename(
        cache.join("collapsible_unwrap"),
        other_cache.join("collapsible_unwrap"),
    )
    .unwrap();
    command(&other_cache)
        .env(env::DYLINT_DRIVER_PATH, &drivers)
        .arg("--no-build")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "`DYLINT_DRIVER_PATH` is deprecated",
        ));
    assert!(!other_cache.join("dylint-driver").exists());
    assert!(!home.path().join(".dylint").exists());
}

#[test]
#[allow(deprecated)]
fn cache_dir_requires_absolute_path() {
    for path in ["", "relative-cache"] {
        cargo_bin_cmd!("cargo-dylint")
            .current_dir("../fixtures/library_packages_in_dylint_toml")
            .env(env::DYLINT_CACHE_DIR, path)
            .env_remove(env::DYLINT_DRIVER_PATH)
            .args(["dylint", "list", "--all", "--no-build"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "`DYLINT_CACHE_DIR` must be an absolute path",
            ));
    }
}
