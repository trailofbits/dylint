use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{MetadataCommand, Package};
use dylint_internal::{clone, env};
use marker_adapter::LintCrateInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::HashMap,
    env::{remove_var, set_var},
    ffi::OsStr,
    fmt::Write as _,
    fs::{read_dir, read_to_string, write, OpenOptions},
    io::Write as _,
    path::Path,
};
use tempfile::tempdir;

#[test]
fn ui() {
    let tempdir = tempdir().unwrap();

    clone_rust_marker(tempdir.path()).unwrap();

    let marker_lint_crates = marker_lint_crates(tempdir.path()).unwrap();

    patch_marker(tempdir.path(), &marker_lint_crates).unwrap();

    // smoelius: Hack. Build the marker Dylint library using the marker repository's target
    // directory. This allows the library to be found when `dylint_testing` is run from within the
    // repository.
    dylint_internal::cargo::build("marker", false)
        .envs([(
            env::CARGO_TARGET_DIR,
            &*tempdir.path().join("target").to_string_lossy(),
        )])
        .success()
        .unwrap();

    dylint_internal::cargo::test("marker", false)
        .current_dir(tempdir.path().join("marker_lints"))
        .args(["--test", "dylint"])
        .success()
        .unwrap();
}

const URL: &str = "https://github.com/rust-marker/marker";

fn clone_rust_marker(path: &Path) -> Result<()> {
    let marker_lints = marker_adapter_package()?;
    clone(URL, &format!("v{}", marker_lints.version), path, false)?;
    Ok(())
}

fn marker_adapter_package() -> Result<Package> {
    let metadata = MetadataCommand::new().exec()?;
    metadata
        .packages
        .into_iter()
        .find(|package| package.name == "marker_adapter")
        .ok_or_else(|| anyhow!("Could not find package"))
}

fn marker_lint_crates(path: &Path) -> Result<String> {
    let mut command = dylint_internal::cargo::run("marker test-setup", false);
    command
        .current_dir(path)
        .args(["--bin", "cargo-marker", "--", "marker", "test-setup"]);
    let output = command.output()?;
    ensure!(output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;

    let mut env_vars: HashMap<_, _> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("env:"))
        .filter_map(|line| line.split_once('='))
        .map(|(var, value)| (var.to_string(), value.to_string()))
        .collect();

    env_vars
        .remove(marker_adapter::LINT_CRATES_ENV)
        .ok_or_else(|| anyhow!("Could not find `{}`", marker_adapter::LINT_CRATES_ENV))
}

fn patch_marker(path: &Path, marker_lint_crates: &str) -> Result<()> {
    add_marker_lint_example(path)?;

    add_marker_lint_test(path, marker_lint_crates)?;

    remove_marker_lint_stderr_line_numbers(path)?;

    Ok(())
}

// smoelius: Hack. Add the `diag_msg_uppercase_start` uitest as an example to the `marker_lints`
// package. That uitest has dependencies, and adding it as an example appears to be the easiest way
// to make it work with `dylint_testing`.
fn add_marker_lint_example(path: &Path) -> Result<()> {
    let mut cargo_toml = OpenOptions::new()
        .append(true)
        .open(path.join("marker_lints/Cargo.toml"))?;

    write!(
        cargo_toml,
        r#"
[dev-dependencies.dylint_testing]
path = "{}"
default-features = false

[[example]]
name = "diag_msg_uppercase_start"
path = "tests/ui/diag_msg_uppercase_start.rs"
"#,
        Path::new("../../../utils/testing")
            .canonicalize()
            .unwrap()
            .to_string_lossy(),
    )?;

    Ok(())
}

fn add_marker_lint_test(path: &Path, marker_lint_crates: &str) -> Result<()> {
    let dylint_toml = dylint_toml(marker_lint_crates)?;

    write(
        path.join("marker_lints/tests/dylint.rs"),
        format!(
            r#"#[test]
fn dylint() {{
    dylint_testing::ui::Test::examples("marker")
        .dylint_toml({dylint_toml:?})
        .run();
}}
"#
        ),
    )?;

    Ok(())
}

fn dylint_toml(marker_lint_crates: &str) -> Result<String> {
    let lint_crates = parse_marker_lint_crates(marker_lint_crates)?;

    let dylint_toml = lint_crates
        .into_iter()
        .map(|LintCrateInfo { name, path }| {
            format!(
                "\
[[marker.lint_crates]]
name = {name:?}
path = {path:?}
"
            )
        })
        .collect();

    Ok(dylint_toml)
}

// smoelius: Hack. Set the environment variable, then call `LintCrateInfo::list_from_env`.
fn parse_marker_lint_crates(marker_lint_crates: &str) -> Result<Vec<LintCrateInfo>> {
    // smoelius: Ensure the variable is not already set.
    env::var(marker_adapter::LINT_CRATES_ENV).unwrap_err();

    set_var(marker_adapter::LINT_CRATES_ENV, marker_lint_crates);

    let lint_crates = LintCrateInfo::list_from_env()?;

    remove_var(marker_adapter::LINT_CRATES_ENV);

    Ok(lint_crates)
}

fn remove_marker_lint_stderr_line_numbers(path: &Path) -> Result<()> {
    for entry in read_dir(path.join("marker_lints/tests/ui"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("stderr")) {
            continue;
        }
        remove_line_numbers(&path)?;
    }
    Ok(())
}

static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*([0-9]+)\s*\|").unwrap());

fn remove_line_numbers(path: &Path) -> Result<()> {
    let input = read_to_string(path)?;
    let mut output = String::new();

    for line in input.lines() {
        if let Some(captures) = RE.captures(line) {
            assert_eq!(2, captures.len());
            let capture = captures.get(1).unwrap();
            writeln!(
                output,
                "{}{}{}",
                &line[..capture.start()],
                "L".repeat(capture.len()),
                &line[capture.end()..]
            )?;
        } else {
            writeln!(output, "{line}")?;
        }
    }

    write(path, output)?;

    Ok(())
}
