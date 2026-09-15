use anyhow::{Context, Result, bail};
use assert_cmd::cargo::cargo_bin_cmd;
use cargo_metadata::Dependency;
use dylint_internal::{CommandExt, env, library_plain_filename, msrv, rustup::SanitizeEnvironment};
use glob::glob;
use predicates::prelude::*;
use semver::Version;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn new_package() {
    for (change_build_dir_location, use_dylint_link) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        #[deny(clippy::unwrap_used)]
        || -> Result<()> {
            let build_dir = if change_build_dir_location {
                let build_dir =
                    tempdir().with_context(|| "Could not create temporary directory")?;
                Some(build_dir)
            } else {
                None
            };

            let tempdir = tempdir().with_context(|| "Could not create temporary directory")?;

            let path_buf = tempdir.path().join("filled_in");

            cargo_bin_cmd!("cargo-dylint")
                .args(["dylint", "new", &path_buf.to_string_lossy(), "--isolate"])
                .assert()
                .success();

            check_dylint_dependencies(&path_buf)?;

            dylint_internal::packaging::use_local_dylint_linting(&path_buf)?;

            let mut command = dylint_internal::cargo::build("filled-in dylint-template").build();
            command.sanitize_environment();
            if let Some(build_dir) = &build_dir {
                command.env(env::CARGO_BUILD_BUILD_DIR, build_dir.path());
            }
            if use_dylint_link {
                command.env(env::DYLINT_LINK_ENABLE_COPY_LIBRARY, "1");
            }
            command.current_dir(&path_buf);
            command.success()?;

            // smoelius: Check that the library was actually built in the configured build
            // directory.
            if let Some(build_dir) = &build_dir {
                let maybe_path_buf = glob(
                    &build_dir
                        .path()
                        .join("debug/build/filled_in/*/out")
                        .join(library_plain_filename("filled_in"))
                        .to_string_lossy(),
                )
                .ok()
                .as_mut()
                .and_then(Iterator::next)
                .transpose()?;
                if maybe_path_buf.is_none() {
                    bail!("Could not find filled-in dylint-template library");
                }
            }

            let mut command = dylint_internal::cargo::test("filled-in dylint-template").build();
            command.sanitize_environment();
            if let Some(build_dir) = &build_dir {
                command.env(env::CARGO_BUILD_BUILD_DIR, build_dir.path());
            }
            if use_dylint_link {
                command.env(env::DYLINT_LINK_ENABLE_COPY_LIBRARY, "1");
            }
            command.current_dir(&path_buf);
            command.success()?;

            if let Some(build_dir) = build_dir {
                drop(build_dir);
            } else {
                assert!(!change_build_dir_location);
            }

            Ok(())
        }()
        .unwrap_or_else(|error| {
            panic!(
                "failed with change_build_dir_location={change_build_dir_location:?}, \
                 use_dylint_link={use_dylint_link:?}: {error:?}"
            );
        });
    }
}

fn check_dylint_dependencies(dir: &Path) -> Result<()> {
    let metadata = dylint_internal::cargo::metadata(dir)?;
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
