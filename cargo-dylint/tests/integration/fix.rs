use anyhow::{Context, Result, anyhow};
use assert_cmd::{cargo::cargo_bin_cmd, prelude::*};
use std::{
    fs::{OpenOptions, read_to_string, write},
    io::Write,
    path::Path,
};
use tempfile::tempdir;

const CATEGORY: &str = "restriction";
const LIB_NAME: &str = "const_path_join";

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

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
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

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--lib", LIB_NAME, "--fix", "--", "--allow-dirty"])
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

    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
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
