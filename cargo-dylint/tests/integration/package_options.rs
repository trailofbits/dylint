use anyhow::{Context, Result, anyhow};
use assert_cmd::prelude::*;
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_internal::{CommandExt, clone, env::enabled, rustup::SanitizeEnvironment};
use predicates::prelude::*;
use regex::Regex;
use semver::Version;
use snapbox::assert_data_eq;
use std::{fs::read_to_string, path::Path, process::Stdio};
use tempfile::tempdir;

const SHORT_ID_LEN: usize = 7;

// smoelius: I expected `git2-0.17.2` to build with nightly-2022-06-30, which corresponds to
// `--rust-version 1.64.0`. I'm not sure why it doesn't.
// smoelius: Dylint's MSRV was recently bumped to 1.68.
// smoelius: `home v0.5.9` (2013-12-15) requires rustc 1.70.0 or newer.
// smoelius: `cargo-util v0.2.7` requires rustc 1.72.0 or newer.
// smoelius: `cargo-platform v0.1.8` requires rustc 1.73 or newer.
// smoelius: `rustfix v0.8.4` requires rustc 1.75 or newer.
// smoelius: `rustfix v0.8.5` requires rustc 1.77 or newer.
// smoelius: `rustfix v0.8.6` requires rustc 1.78 or newer. However, I get errors building
// `serde` 1.0.210 with rustc 1.78, and `proc_macro2` 1.0.87 with rustc 1.79. So I am bumping
// `RUSTC_VERSION` to 1.80.
// smoelius: `home@0.5.11` (2024-12-16) requires rustc 1.81.
// smoelius: `icu_collections@2.0.0` and several other packages require rustc 1.82.
// smoelius: Edition 2024 was stabilized with Rust 1.85.
const RUST_VERSION: &str = "1.86.0";

#[test]
fn new_package() {
    let tempdir = tempdir().unwrap();

    let path_buf = tempdir.path().join("filled_in");

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
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
    let rust_version = Version::parse(RUST_VERSION).unwrap();

    let upgrade = || {
        let mut command = std::process::Command::cargo_bin("cargo-dylint").unwrap();
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

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
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

const DYLINT_URL: &str = "https://github.com/trailofbits/dylint";

// smoelius: Each of the following commits is just before an "Upgrade examples" commit. In the
// upgrades, the changes to the `restriction` lints tend to be small. Thus, the auto-correct option
// should be able to generate fixes for many of them.
const REVS_AND_RUST_VERSIONS: &[(&str, &str, bool)] = &[
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/53e617e844b1f2c0d953d67b47a525381ec094c7
    ("5343770f654d2bcd0fe246bb333e1a2b63048df0", "1.92.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/5d5717e3e314c8bdb1f6d0bcd3852d1059a2b482
    ("1e03ecf75981d94a7917866454b7bd0214916165", "1.91.0", true),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/c45f27a8068ff9de1efa88f7fde1574dd04ed8c2
    ("80ea87d485a80c61efc65ab49dd7b28b54554969", "1.90.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/24407a3d328ad0d2f6318ba186b2ac126713622f
    ("a93c166d88662b6bfc29ca7c31177a6c6f2b897b", "1.89.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/33969746aef6947c68d7adb55137ce8a13d9cc47
    ("5b3792515ac255fdb06a31b10eb3c9f7949a3ed5", "1.80.0", true),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/7bc453f0778dee3b13bc1063773774304ac96cad
    ("23c08c8a0b043d26f66653bf173a0e6722a2d699", "1.79.0", true),
];

#[test]
fn upgrade_with_auto_correct() {
    for &(rev, rust_version, should_succeed) in REVS_AND_RUST_VERSIONS {
        let short_id = &rev[..SHORT_ID_LEN];
        let stderr_path = Path::new("tests/integration/auto_correct")
            .join(short_id)
            .with_extension("stderr");
        let expected_stderr = read_to_string(stderr_path).unwrap();

        let tempdir = tempdir().unwrap();

        clone(DYLINT_URL, rev, tempdir.path(), false).unwrap();

        let mut command = std::process::Command::cargo_bin("cargo-dylint").unwrap();
        command.args([
            "dylint",
            "upgrade",
            &tempdir
                .path()
                .join("examples/restriction")
                .to_string_lossy(),
            "--auto-correct",
            "--rust-version",
            rust_version,
        ]);
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::piped());

        let output = command.output().unwrap();

        assert_data_eq!(output.stderr, expected_stderr);

        if enabled("DEBUG_DIFF") {
            let mut command = std::process::Command::new("git");
            command.current_dir(&tempdir);
            command.args(["--no-pager", "diff", "--", "*.rs"]);
            command.success().unwrap();
        }

        let status = dylint_internal::cargo::check("auto-corrected, upgraded library package")
            .build()
            .sanitize_environment()
            .current_dir(tempdir.path().join("examples/restriction"))
            .env("RUSTFLAGS", "--allow=warnings")
            .arg("--quiet")
            .status()
            .unwrap();

        assert_eq!(should_succeed, status.success());
    }
}

#[allow(dead_code)]
fn rust_version(path: &Path) -> Result<Version> {
    let re = Regex::new(r#"^clippy_utils = .*\btag = "rust-([^"]*)""#).unwrap();
    let manifest = path.join("Cargo.toml");
    let contents = read_to_string(&manifest).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            manifest.to_string_lossy()
        )
    })?;
    let rust_version = contents
        .lines()
        .find_map(|line| re.captures(line).map(|captures| captures[1].to_owned()))
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))?;
    Version::parse(&rust_version).map_err(Into::into)
}
