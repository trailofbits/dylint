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
            let split_toolchain = rustup_toolchain.split('-');
            let count = split_toolchain.clone().count();
            // MinerSebas: Replace with std version of intersperse, once it is stabilized: https://github.com/rust-lang/rust/issues/79524
            let trimed_toolchain: String =
                itertools::Itertools::intersperse(split_toolchain.skip(count - 4), "-").collect();

            path = cc::windows_registry::find_tool(trimed_toolchain.as_str(), "link.exe")
                .ok_or_else(|| anyhow!("Could not find the MSVC Linker"))?
                .path()
                .into();
        } else {
            return Err(anyhow!("Only the MSVC toolchain is supported on Windows."));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        path = "cc".into();
    };

    let args: Vec<String> = std::env::args().collect();

    Command::new(path).args(&args[1..]).success()?;

    let mut args = std::env::args();

    #[allow(clippy::redundant_else)]
    if rustup_toolchain.ends_with("msvc") {
        #[cfg(target_os = "windows")]
        {
            for arg in args {
                if arg.starts_with("/OUT:") || arg.starts_with('@') {
                    let path: PathBuf = if arg.starts_with("/OUT:") {
                        arg.trim_start_matches("/OUT:").into()
                    } else {
                        extract_out_path_from_linker_response_file_msvc(
                            arg.trim_start_matches('@'),
                        )?
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
fn extract_out_path_from_linker_response_file_msvc(path: impl AsRef<Path>) -> Result<PathBuf> {
    // On Windows the cmd line has a Limit of 8191 Characters.
    // If your command would exceed this you can instead use a Linker Response File to set arguments.
    // (https://docs.microsoft.com/en-us/cpp/build/reference/at-specify-a-linker-response-file?view=msvc-160)

    // Read the Linker Response File
    let mut buf: Vec<u8> = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    // Convert the File from UTF-16 to a Rust UTF-8 String
    // (Only necessary for MSVC, the GNU Linker uses UTF-8 isntead.)
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
