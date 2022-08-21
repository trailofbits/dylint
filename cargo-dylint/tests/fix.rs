use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use std::{
    fs::{read_to_string, write, OpenOptions},
    io::Write,
    path::Path,
};
use tempfile::tempdir;
use test_log::test;

const CATEGORY: &str = "restriction";
const LIB_NAME: &str = "path_separator_in_string_literal";

fn workspace_metadata(path_spec: &str) -> String {
    format!(
        r#"
[workspace.metadata.dylint]
libraries = [
    {{ path = "{}" }},
]
"#,
        path_spec,
    )
}

#[cfg(target_os = "windows")]
const MAIN_RS: &str = r#"
fn main() {
    let _ = std::path::Path::new("..\\target");
    let _ = std::path::PathBuf::from("..\\target");
}
"#;

#[cfg(not(target_os = "windows"))]
const MAIN_RS: &str = r#"
fn main() {
    let _ = std::path::Path::new("../target");
    let _ = std::path::PathBuf::from("../target");
}
"#;

const MAIN_FIXED: &str = r#"
fn main() {
    let _ = std::path::Path::new("..").join("target").as_path();
    let _ = std::path::PathBuf::from("..").join("target");
}
"#;

#[test]
fn fix() {
    let tempdir = tempdir().unwrap();

    std::process::Command::new("cargo")
        .current_dir(&tempdir)
        .args(&[
            "init",
            "--edition=2018",
            "--name",
            tempdir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches('.'),
        ])
        .assert()
        .success();

    append_workspace_metadata(tempdir.path()).unwrap();

    write(tempdir.path().join("src").join("main.rs"), MAIN_RS).unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(&tempdir)
        .args(&["dylint", "--fix", LIB_NAME])
        .assert()
        .success();

    let main_actual = read_to_string(tempdir.path().join("src").join("main.rs")).unwrap();

    assert_eq!(main_actual, MAIN_FIXED);
}

#[allow(unknown_lints)]
#[allow(env_cargo_path)]
fn append_workspace_metadata(path: &Path) -> Result<()> {
    let manifest = path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory"))?;

    let path_spec = parent
        .join("examples")
        .join(CATEGORY)
        .join(LIB_NAME)
        .to_string_lossy()
        .replace('\\', "\\\\");

    writeln!(file, "{}", workspace_metadata(&path_spec))
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;

    Ok(())
}
