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
    ffi::OsStr,
    fs::copy,
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    Command::new("cc").args(&args[1..]).success()?;

    let cargo_pkg_name = var(env::CARGO_PKG_NAME)?;
    let rustup_toolchain = var(env::RUSTUP_TOOLCHAIN)?;

    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "-o" {
            if_chain! {
                if let Some(path) = args.next();
                let path = Path::new(&path);
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
            break;
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
