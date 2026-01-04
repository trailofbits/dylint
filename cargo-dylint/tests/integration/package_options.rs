use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_internal::{CommandExt, msrv, rustup::SanitizeEnvironment};
use predicates::prelude::*;
use semver::Version;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path_buf = tempdir.path().join("filled_in");

    cargo_bin_cmd!("cargo-dylint")
        .args(["dylint", "new", &path_buf.to_string_lossy(), "--isolate"])
        .assert()
        .success();

    check_dylint_dependencies(&path_buf).unwrap();

    dylint_internal::packaging::use_local_packages(&path_buf).unwrap();

    dylint_internal::cargo::build("filled-in dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&path_buf)
        .success()
        .unwrap();

    dylint_internal::cargo::test("filled-in dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&path_buf)
        .success()
        .unwrap();
}

fn check_dylint_dependencies(path: &Path) -> Result<()> {
    let metadata = MetadataCommand::new().current_dir(path).no_deps().exec()?;
    for package in metadata.packages {
        for Dependency { name: dep, req, .. } in &package.dependencies {
            if dep.starts_with("dylint") {
                if package.name.as_str() == "filled_in" {
                    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                    assert!(req.matches(&version));
                } else {
                    assert_eq!("^".to_owned() + env!("CARGO_PKG_VERSION"), req.to_string());
                }
            }
        }
    }
    Ok(())
}

#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
#[test]
fn downgrade_upgrade_package() {
    let tempdir = tempdir().unwrap();

    dylint_internal::testing::new_template(tempdir.path()).unwrap();

    // smoelius: I broke this downgrading code when I switched dylint-template from using a git tag
    // to a git revision to refer to `clippy_utils`. For now, just hardcode the downgrade version.
    /* let mut rust_version = rust_version(tempdir.path()).unwrap();
    assert!(rust_version.minor != 0);
    rust_version.minor -= 1; */
    let rust_version = Version::parse(msrv::MSRV).unwrap();

    let upgrade = || {
        let mut command = cargo_bin_cmd!("cargo-dylint");
        command.args([
            "dylint",
            "upgrade",
            &tempdir.path().to_string_lossy(),
            "--rust-version",
            &rust_version.to_string(),
        ]);
        command
    };

    upgrade()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refusing to downgrade toolchain"));

    upgrade().args(["--allow-downgrade"]).assert().success();

    dylint_internal::cargo::build("downgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    dylint_internal::cargo::test("downgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .args(["dylint", "upgrade", &tempdir.path().to_string_lossy()])
        .assert()
        .success();

    // smoelius: Temporarily disable the rest of this test because of:
    // https://github.com/dtolnay/proc-macro2/issues/451
    if cfg!(all()) {
        return;
    }

    dylint_internal::cargo::build("upgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    dylint_internal::cargo::test("upgraded dylint-template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();
}
