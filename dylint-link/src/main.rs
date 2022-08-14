#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

#[cfg(target_os = "windows")]
use anyhow::ensure;
use anyhow::{anyhow, Context, Result};
use dylint_internal::{cargo::cargo_home, env, library_filename, Command};
use if_chain::if_chain;
use std::{
    env::{args, consts},
    ffi::OsStr,
    fs::{copy, read_to_string},
    path::{Path, PathBuf},
};
#[cfg(target_os = "windows")]
use std::{fs::File, io::Read};
use toml_edit::{Document, Item};

fn main() -> Result<()> {
    env_logger::init();

    let linker = linker()?;
    let args: Vec<String> = args().collect();
    Command::new(linker).args(&args[1..]).success()?;

    if let Some(path) = output_path(args.iter())? {
        copy_library(&path)?;
    }

    Ok(())
}

fn linker() -> Result<PathBuf> {
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    let target = parse_toolchain(&rustup_toolchain)
        .map_or_else(|| env!("TARGET").to_owned(), |(_, target)| target);
    let cargo_home = cargo_home()?;
    let config_toml = cargo_home.join("config.toml");
    if config_toml.is_file() {
        let file = read_to_string(&config_toml).with_context(|| {
            format!(
                "`read_to_string` failed for `{}`",
                config_toml.to_string_lossy()
            )
        })?;
        let document = file.parse::<Document>()?;
        document
            .as_table()
            .get("target")
            .and_then(Item::as_table)
            .and_then(|table| table.get(&target))
            .and_then(Item::as_table)
            .and_then(|table| table.get("linker"))
            .and_then(Item::as_str)
            .map_or_else(default_linker, |s| Ok(PathBuf::from(s)))
    } else {
        default_linker()
    }
}

#[cfg(target_os = "windows")]
fn default_linker() -> Result<PathBuf> {
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    if rustup_toolchain.split('-').last() == Some("msvc") {
        // MinerSebas: Removes the Release Information: "nightly-2021-04-08-x86_64-pc-windows-msvc" -> "x86_64-pc-windows-msvc"
        // smoelius: The approach has changed slightly.
        if let Some(tool) = parse_toolchain(&rustup_toolchain)
            .and_then(|(_, target)| cc::windows_registry::find_tool(&target, "link.exe"))
        {
            Ok(tool.path().into())
        } else {
            Err(anyhow!("Could not find the MSVC Linker"))
        }
    } else {
        Err(anyhow!("Only the MSVC toolchain is supported on Windows"))
    }
}

#[cfg(not(target_os = "windows"))]
#[allow(clippy::unnecessary_wraps)]
fn default_linker() -> Result<PathBuf> {
    Ok(PathBuf::from("cc"))
}

#[cfg(target_os = "windows")]
fn output_path<'a, I>(iter: I) -> Result<Option<PathBuf>>
where
    I: Iterator<Item = &'a String>,
{
    for arg in iter {
        if let Some(path) = arg.strip_prefix("/OUT:") {
            return Ok(Some(path.into()));
        }
        if let Some(path) = arg.strip_prefix('@') {
            return extract_out_path_from_linker_response_file(path);
        }
    }

    Ok(None)
}

#[cfg(not(target_os = "windows"))]
#[allow(clippy::unnecessary_wraps)]
fn output_path<'a, I>(mut iter: I) -> Result<Option<PathBuf>>
where
    I: Iterator<Item = &'a String>,
{
    while let Some(arg) = iter.next() {
        if arg == "-o" {
            if let Some(path) = iter.next() {
                return Ok(Some(path.into()));
            }
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn extract_out_path_from_linker_response_file(path: impl AsRef<Path>) -> Result<Option<PathBuf>> {
    // MinerSebas: On Windows the cmd line has a Limit of 8191 Characters.
    // If your command would exceed this you can instead use a Linker Response File to set arguments.
    // (https://docs.microsoft.com/en-us/cpp/build/reference/at-specify-a-linker-response-file?view=msvc-160)

    // MinerSebas: Read the Linker Response File
    let mut buf: Vec<u8> = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    // MinerSebas: Convert the File from UTF-16 to a Rust UTF-8 String
    // (Only necessary for MSVC, the GNU Linker uses UTF-8 isntead.)
    // Based on: https://stackoverflow.com/a/57172592
    let file: Vec<u16> = buf
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    let file = String::from_utf16_lossy(file.as_slice());

    let paths: Vec<_> = file
        .lines()
        .flat_map(|line| line.trim().trim_matches('"').strip_prefix("/OUT:"))
        .collect();

    ensure!(paths.len() <= 1, "Found multiple output paths");

    // smoelius: Do not raise an error if no output path is found.
    Ok(paths.last().map(Into::into))
}

fn copy_library(path: &Path) -> Result<()> {
    if_chain! {
        if let Some(lib_name) = parse_path_plain_filename(path);
        let cargo_pkg_name = env::var(env::CARGO_PKG_NAME)?;
        if lib_name == cargo_pkg_name.replace('-', "_");
        then {
            let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
            let filename_with_toolchain = library_filename(&lib_name, &rustup_toolchain);
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("Could not get parent directory"))?;
            let path_with_toolchain = strip_deps(parent).join(filename_with_toolchain);
            copy(&path, &path_with_toolchain).with_context(|| {
                format!(
                    "Could not copy `{}` to `{}`",
                    path.to_string_lossy(),
                    path_with_toolchain.to_string_lossy()
                )
            })?;
        }
    }

    Ok(())
}

// smoelius: I do not know what the right/best way to parse a toolchain is. `parse_toolchain` does
// so by looking for the architecture.
fn parse_toolchain(toolchain: &str) -> Option<(String, String)> {
    let split_toolchain: Vec<_> = toolchain.split('-').collect();
    split_toolchain
        .iter()
        .rposition(|s| ARCHITECTURES.binary_search(s).is_ok())
        .map(|i| {
            (
                split_toolchain[..i].join("-"),
                split_toolchain[i..].join("-"),
            )
        })
}

fn parse_path_plain_filename(path: &Path) -> Option<String> {
    let filename = path.file_name()?;
    let s = filename.to_string_lossy();
    let file_stem = s.strip_suffix(consts::DLL_SUFFIX)?;
    let lib_name = file_stem.strip_prefix(consts::DLL_PREFIX)?;
    Some(lib_name.to_owned())
}

fn strip_deps(path: &Path) -> PathBuf {
    if path.file_name() == Some(OsStr::new("deps")) {
        path.parent()
    } else {
        None
    }
    .unwrap_or(path)
    .to_path_buf()
}

// smoelius: `ARCHITECTURES` is based on: https://doc.rust-lang.org/rustc/platform-support.html
const ARCHITECTURES: &[&str] = &[
    "aarch64",
    "aarch64_be",
    "arm",
    "arm64_32",
    "armebv7r",
    "armv4t",
    "armv5te",
    "armv6",
    "armv6k",
    "armv7",
    "armv7a",
    "armv7k",
    "armv7r",
    "armv7s",
    "asmjs",
    "avr",
    "bpfeb",
    "bpfel",
    "hexagon",
    "i386",
    "i586",
    "i686",
    "m68k",
    "mips",
    "mips64",
    "mips64el",
    "mipsel",
    "mipsisa32r6",
    "mipsisa32r6el",
    "mipsisa64r6",
    "mipsisa64r6el",
    "msp430",
    "nvptx64",
    "powerpc",
    "powerpc64",
    "powerpc64le",
    "riscv32gc",
    "riscv32i",
    "riscv32im",
    "riscv32imac",
    "riscv32imc",
    "riscv64gc",
    "riscv64imac",
    "s390x",
    "sparc",
    "sparc64",
    "sparcv9",
    "thumbv4t",
    "thumbv6m",
    "thumbv7a",
    "thumbv7em",
    "thumbv7m",
    "thumbv7neon",
    "thumbv8m.base",
    "thumbv8m.main",
    "wasm32",
    "wasm64",
    "x86_64",
];

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::{env, ARCHITECTURES};
    use assert_cmd::prelude::*;
    use dylint_internal::packaging::isolate;
    use predicates::prelude::*;
    use std::fs::{create_dir, write};
    use tempfile::{tempdir, tempdir_in};
    use test_log::test;

    #[test]
    fn architectures_are_current() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "target-list"])
            .unwrap();
        let mut architectures = std::str::from_utf8(&output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_once('-').map(|(architecture, _)| architecture))
            .collect::<Vec<_>>();
        architectures.sort_unstable();
        architectures.dedup();
        assert_eq!(ARCHITECTURES, architectures);
    }

    #[test]
    fn architectures_are_sorted() {
        let mut architectures = ARCHITECTURES.to_vec();
        architectures.sort_unstable();
        architectures.dedup();
        assert_eq!(ARCHITECTURES, architectures);
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[cfg_attr(
        dylint_lib = "non_thread_safe_call_in_test",
        allow(non_thread_safe_call_in_test)
    )]
    #[test]
    fn global_config() {
        let cargo_home = tempdir().unwrap();
        let package = tempdir_in(".").unwrap();

        dylint_internal::cargo::build("dylint-link", false)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .success()
            .unwrap();

        dylint_internal::cargo::init("package `global_config_test`", false)
            .current_dir(package.path())
            .args(&["--name", "global_config_test"])
            .success()
            .unwrap();

        isolate(package.path()).unwrap();

        let package_cargo = package.path().join(".cargo");
        create_dir(&package_cargo).unwrap();
        write(
            package_cargo.join("config.toml"),
            r#"
[target.x86_64-unknown-linux-gnu]
linker = "../../target/debug/dylint-link"
"#,
        )
        .unwrap();

        std::process::Command::new("cargo")
            .current_dir(package.path())
            .arg("build")
            .assert()
            .success();

        write(
            cargo_home.path().join("config.toml"),
            r#"
[target.x86_64-unknown-linux-gnu]
linker = "false"
"#,
        )
        .unwrap();

        std::process::Command::new("cargo")
            .current_dir(package.path())
            .arg("clean")
            .assert()
            .success();

        std::process::Command::new("cargo")
            .env(env::CARGO_HOME, cargo_home.path())
            .current_dir(package.path())
            .arg("build")
            .assert()
            .failure()
            .stderr(
                predicate::str::is_match(
                    "error: linking with `[^`]*/target/debug/dylint-link` failed",
                )
                .unwrap(),
            );
    }
}
