use anyhow::{anyhow, Context, Result};
use assert_cmd::prelude::*;
use std::{
    fs::{read_to_string, write, OpenOptions},
    io::Write,
    path::Path,
};
use tempfile::tempdir;

const CATEGORY: &str = "restriction";
const LIB_NAME: &str = "const_path_join";

fn workspace_metadata(path_spec: &str) -> String {
    format!(
        r#"
[workspace.metadata.dylint]
libraries = [
    {{ path = "{path_spec}" }},
]
"#,
    )
}

const MAIN_RS: &str = r#"
fn main() {
    let _ = std::path::Path::new("..").join("target");
    let _ = std::path::PathBuf::from("..").join("target");
}
"#;

const MAIN_FIXED: &str = r#"
fn main() {
    let _ = std::path::PathBuf::from("../target");
    let _ = std::path::PathBuf::from("../target");
}
"#;

#[test]
fn fix() {
    let tempdir = tempdir().unwrap();

    std::process::Command::new("cargo")
        .current_dir(&tempdir)
        .args([
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

    write(tempdir.path().join("src/main.rs"), MAIN_RS).unwrap();

    std::process::Command::cargo_bin("cargo-dylint")
        .unwrap()
        .current_dir(&tempdir)
        .args(["dylint", "--fix", LIB_NAME, "--", "--allow-dirty"])
        .assert()
        .success();

    let main_actual = read_to_string(tempdir.path().join("src/main.rs")).unwrap();

    assert_eq!(MAIN_FIXED, main_actual);
}

fn append_workspace_metadata(path: &Path) -> Result<()> {
    let manifest = path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    #[allow(unknown_lints, env_cargo_path)]
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
