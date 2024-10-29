use anyhow::{anyhow, ensure, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    thread,
};

fn main() -> Result<()> {
    let toolchains = collect_toolchains(&["cargo-dylint", "examples", "internal"])?;

    println!("{:#?}", &toolchains);

    let handles = std::iter::once("nightly".to_owned())
        .chain(toolchains)
        .map(|toolchain| (thread::spawn(move || install_toolchain(&toolchain))));

    for handle in handles {
        let () = handle
            .join()
            .map_err(|error| anyhow!("{error:?}"))
            .and_then(std::convert::identity)?;
    }

    Ok(())
}

fn collect_toolchains(dirs: &[&str]) -> Result<Vec<String>> {
    let mut toolchains = Vec::new();

    for dir in dirs {
        let toolchains_for_dir = collect_toolchains_for_dir(dir)?;
        toolchains.extend(toolchains_for_dir);
    }

    toolchains.sort();
    toolchains.dedup();

    Ok(toolchains)
}

fn collect_toolchains_for_dir(dir: &str) -> Result<Vec<String>> {
    let mut ls_files = Command::new("git")
        .args(["ls-files", dir])
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| "Could not spawn `git ls-files`")?;

    let stdout = ls_files.stdout.take().unwrap();
    BufReader::new(stdout)
        .lines()
        .try_fold(Vec::new(), |mut toolchains, result| -> Result<_> {
            let path = result.with_context(|| "Could not read from `git ls-files`")?;
            let toolchains_for_path = collect_toolchains_for_path(path)?;
            toolchains.extend(toolchains_for_path);
            Ok(toolchains)
        })
}

static RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\<nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}\>").unwrap());

fn collect_toolchains_for_path(path: impl AsRef<Path>) -> Result<Vec<String>> {
    let file = File::open(&path).with_context(|| format!("Could not open {:?}", path.as_ref()))?;
    BufReader::new(file)
        .lines()
        .try_fold(Vec::new(), |mut toolchains, result| -> Result<_> {
            let line =
                result.with_context(|| format!("Could not read from {:?}", path.as_ref()))?;
            let n = line.find("//").unwrap_or(line.len());
            toolchains.extend(RE.find_iter(&line[..n]).map(|m| m.as_str().to_owned()));
            Ok(toolchains)
        })
}

fn install_toolchain(toolchain: &str) -> Result<()> {
    let status = Command::new("rustup")
        .args([
            "install",
            toolchain,
            "--profile=minimal",
            "--no-self-update",
        ])
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
