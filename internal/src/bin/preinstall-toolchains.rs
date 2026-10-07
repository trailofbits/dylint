use anyhow::{Context, Result, anyhow, ensure};
use regex::Regex;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
    thread,
};

// smoelius: As stated in .github/workflows/ci.yml, the purpose of `preinstall-toolchains` is to
// avoid using `rustup` concurrently. Such concurrent use arises from tests. Thus, we need only
// consider cases where multiple tests use the same toolchain. This excludes the `expensive` tests,
// for example.
const DIRS: &[&str] = &["cargo-dylint", "examples", "internal"];

fn main() -> Result<()> {
    let toolchains = collect_toolchains()?;

    let (kept, discarded) = filter_toolchains(toolchains);

    println!("{kept:#?}");

    println!(
        "discarded: {:#?}",
        discarded
            .into_iter()
            .map(|(toolchain, path)| format!("({toolchain}, {})", path.display()))
            .collect::<Vec<_>>()
    );

    let mut handles = Vec::new();
    for toolchain in ["stable", "nightly"] {
        handles.push(thread::spawn(move || install_toolchain(toolchain)));
    }
    for toolchain in kept {
        handles.push(thread::spawn(move || install_toolchain(toolchain)));
    }

    for handle in handles {
        let () = handle
            .join()
            .map_err(|error| anyhow!("{error:?}"))
            .and_then(std::convert::identity)?;
    }

    Ok(())
}

fn collect_toolchains() -> Result<Vec<(String, PathBuf)>> {
    let mut ls_files = Command::new("git")
        .arg("ls-files")
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| "Could not spawn `git ls-files`")?;

    let stdout = ls_files.stdout.take().unwrap();
    let mut toolchains = Vec::new();
    for result in BufReader::new(stdout).lines() {
        let path = result
            .map(PathBuf::from)
            .with_context(|| "Could not read from `git ls-files`")?;
        if path
            .file_stem()
            .and_then(OsStr::to_str)
            .is_some_and(|file_stem| file_stem.ends_with("_no_preinstall"))
        {
            continue;
        }
        let toolchains_for_path = collect_toolchains_for_path(&path)?;
        toolchains.extend(
            toolchains_for_path
                .into_iter()
                .map(|toolchain| (toolchain, path.clone())),
        );
    }
    Ok(toolchains)
}

static RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\<nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}\>").unwrap());

fn collect_toolchains_for_path(path: impl AsRef<Path>) -> Result<Vec<String>> {
    let file = File::open(&path)
        .with_context(|| format!("Could not open `{}`", path.as_ref().display()))?;
    let mut toolchains = Vec::new();
    for result in BufReader::new(file).lines() {
        let line = match result {
            Ok(line) => line,
            Err(error) => {
                eprintln!("Could not read from `{}`: {error}", path.as_ref().display());
                return Ok(Vec::new());
            }
        };
        let n = line.find("//").unwrap_or(line.len());
        toolchains.extend(RE.find_iter(&line[..n]).map(|m| m.as_str().to_owned()));
    }
    Ok(toolchains)
}

fn filter_toolchains(toolchains: Vec<(String, PathBuf)>) -> (Vec<String>, Vec<(String, PathBuf)>) {
    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    for (toolchain, path) in toolchains {
        if DIRS.iter().any(|dir| path.starts_with(dir))
            && (path.file_name() == Some(OsStr::new("rust-toolchain.toml"))
                || path.extension() == Some(OsStr::new("rs")))
        {
            kept.push(toolchain);
            continue;
        }

        discarded.push((toolchain, path));
    }

    kept.sort();
    kept.dedup();

    // smoelius: If a toolchain will be installed, do not call it "discarded".
    discarded.retain(|(toolchain, _)| kept.binary_search(toolchain).is_err());

    discarded.sort();
    discarded.dedup();

    (kept, discarded)
}

fn install_toolchain(toolchain: impl AsRef<str>) -> Result<()> {
    let toolchain = toolchain.as_ref();

    let status = Command::new("rustup")
        .args(["install", toolchain, "--profile=minimal"])
        .status()
        .with_context(|| format!("Could not install {toolchain} with `rustup`"))?;
    ensure!(status.success());

    let status = Command::new("rustup")
        .args([
            "component",
            "add",
            "llvm-tools-preview",
            "rustc-dev",
            "--toolchain",
            toolchain,
        ])
        .status()
        .with_context(|| format!("Could not add components to {toolchain} with `rustup`"))?;
    ensure!(status.success());

    Ok(())
}
