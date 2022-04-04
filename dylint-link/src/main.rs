#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

#[cfg(target_os = "windows")]
use anyhow::ensure;
use anyhow::{anyhow, Context, Result};
use dylint_internal::{
    env::{self, var},
    library_filename, Command,
};
use if_chain::if_chain;
use std::{
    env::consts,
    ffi::OsStr,
    fs::copy,
    path::{Path, PathBuf},
};
#[cfg(target_os = "windows")]
use std::{fs::File, io::Read};

fn main() -> Result<()> {
    env_logger::init();

    let linker = linker()?;
    let args: Vec<String> = std::env::args().collect();
    Command::new(linker).args(&args[1..]).success()?;

    if let Some(path) = output_path(args.iter())? {
        copy_library(&path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn linker() -> Result<PathBuf> {
    let rustup_toolchain = var(env::RUSTUP_TOOLCHAIN)?;
    let split_toolchain: Vec<_> = rustup_toolchain.split('-').collect();
    if_chain! {
        if split_toolchain.last() == Some(&"msvc");
        let len = split_toolchain.len();
        if len >= 4;
        then {
            // MinerSebas: Removes the Release Information: "nightly-2021-04-08-x86_64-pc-windows-msvc" -> "x86_64-pc-windows-msvc"
            let trimmed_toolchain: String = split_toolchain[len - 4..].join("-");
            if let Some(tool) = cc::windows_registry::find_tool(&trimmed_toolchain, "link.exe") {
                Ok(tool.path().into())
            } else {
                Err(anyhow!("Could not find the MSVC Linker"))
            }
        } else {
            Err(anyhow!("Only the MSVC toolchain is supported on Windows"))
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[allow(clippy::unnecessary_wraps)]
fn linker() -> Result<PathBuf> {
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
        if let Some(lib_name) = parse_path(path);
        let cargo_pkg_name = var(env::CARGO_PKG_NAME)?;
        if lib_name == cargo_pkg_name.replace('-', "_");
        then {
            let rustup_toolchain = var(env::RUSTUP_TOOLCHAIN)?;
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

fn parse_path(path: &Path) -> Option<String> {
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
