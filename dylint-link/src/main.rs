#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

use anyhow::{anyhow, Result};
use dylint_internal::{
    env::{self, var},
    Command,
};
use if_chain::if_chain;
use std::{
    env::consts,
    ffi::{OsStr, OsString},
    fs::copy,
    path::{Path, PathBuf},
};
#[cfg(target_os = "windows")]
use std::{fs::File, io::Read};

fn main() -> Result<()> {
    env_logger::init();

    let cargo_pkg_name = var(env::CARGO_PKG_NAME)?;
    let rustup_toolchain = var(env::RUSTUP_TOOLCHAIN)?;

    let path: OsString;

    #[cfg(target_os = "windows")]
    {
        if rustup_toolchain.ends_with("msvc") {
            // Removes the Release Information: "nightly-2021-04-08-x86_64-pc-windows-msvc" -> "x86_64-pc-windows-msvc"
            let trimed_toolchain = {
                let split_toolchain = rustup_toolchain.split('-');

                let count = split_toolchain.clone().count();

                if count == 4 {
                    rustup_toolchain.to_owned()
                } else {
                    let mut temp = String::new();
                    for part in split_toolchain.skip(count - 4) {
                        if !temp.is_empty() {
                            temp.push('-')
                        }

                        temp.push_str(part);
                    }

                    temp
                }
            };

            path = cc::windows_registry::find_tool(trimed_toolchain.as_str(), "link.exe")
                .ok_or_else(|| anyhow!("Could not find the MSVC Linker"))?
                .path()
                .into();
        } else {
            path = "cc".into();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        path = "cc".into();
    };

    let args: Vec<String> = std::env::args().collect();

    Command::new(path).args(&args[1..]).success()?;

    let mut args = std::env::args();

    if rustup_toolchain.ends_with("msvc") {
        #[cfg(target_os = "windows")]
        {
            while let Some(arg) = args.next() {
                if arg.starts_with("/OUT:") || arg.starts_with('@') {
                    let path: PathBuf = if arg.starts_with("/OUT:") {
                        arg.trim_start_matches("/OUT:").into()
                    } else {
                        extract_out_path_from_linker_response_file(arg.trim_start_matches('@'))?
                    };

                    copy_library(
                        path.as_path(),
                        cargo_pkg_name.as_str(),
                        rustup_toolchain.as_str(),
                    )?;
                    break;
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            return Err(anyhow!(
                "dylint-link can only link with the MSVC toolchain on Windows"
            ));
        }
    } else {
        while let Some(arg) = args.next() {
            if arg == "-o" {
                if let Some(path) = args.next() {
                    let path: OsString = path.into();
                    copy_library(
                        Path::new(&path),
                        cargo_pkg_name.as_str(),
                        rustup_toolchain.as_str(),
                    )?;
                }
                break;
            }
        }
    };

    Ok(())
}

#[cfg(target_os = "windows")]
fn extract_out_path_from_linker_response_file(path: impl AsRef<Path>) -> Result<PathBuf> {
    // The MSVC Linker can also accept Arguments through a Linker Response File
    // (https://docs.microsoft.com/en-us/cpp/build/reference/at-specify-a-linker-response-file?view=msvc-160)

    // Read the Linker Response File
    let mut buf: Vec<u8> = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    // Convert the File from UTF-16 to a Rust UTF-8 String
    let file: Vec<u16> = buf
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    let file = String::from_utf16_lossy(file.as_slice()).replace("\\\"", "\"");

    let lines = file
        .trim_start_matches('\"')
        .trim_end_matches('\"')
        .split("\"\n\"");

    lines
        .filter(|line| line.starts_with("/OUT:"))
        .map(|line| line.trim_start_matches("/OUT:"))
        .next()
        .map(|path| path.into())
        .ok_or_else(|| anyhow!("Malformed out path flag"))
}

fn copy_library(path: &Path, cargo_pkg_name: &str, rustup_toolchain: &str) -> Result<()> {
    if_chain! {
        if let Some(lib_name) = parse_path(path);
        if lib_name == cargo_pkg_name.replace("-", "_");
        then {
            let filename_with_toolchain = format!(
                "{}{}@{}{}",
                consts::DLL_PREFIX,
                lib_name,
                rustup_toolchain,
                consts::DLL_SUFFIX
            );
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("Could not get parent directory"))?;
            let path_with_toolchain = strip_deps(parent).join(filename_with_toolchain);
            copy(path, path_with_toolchain)?;
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
