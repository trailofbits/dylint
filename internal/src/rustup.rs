use crate::{CommandExt, env};
use anyhow::{Result, anyhow};
use cargo_metadata::MetadataCommand;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

// smoelius: Should this be merged into `CommandExt`?
pub trait SanitizeEnvironment {
    fn sanitize_environment(&mut self) -> &mut Self;
}

impl SanitizeEnvironment for Command {
    fn sanitize_environment(&mut self) -> &mut Self {
        self.env_remove(env::CARGO);
        self.env_remove(env::RUSTC);
        self.env_remove(env::RUSTUP_TOOLCHAIN);
        self
    }
}

impl SanitizeEnvironment for MetadataCommand {
    fn sanitize_environment(&mut self) -> &mut Self {
        self.env_remove(env::CARGO);
        self.env_remove(env::RUSTC);
        self.env_remove(env::RUSTUP_TOOLCHAIN);
        self
    }
}

// smoelius: Consider carefully whether you need to call this function! In most cases, the toolchain
// you want is not the one returned by rustup.
pub fn active_toolchain(path: &Path) -> Result<String> {
    let output = Command::new("rustup")
        .sanitize_environment()
        .current_dir(path)
        .args(["show", "active-toolchain"])
        .logged_output(true)?;
    let stdout = std::str::from_utf8(&output.stdout)?;

    // split at the first whitespace character
    parse_active_toolchain(stdout)
}

// Split from the first whitespace character
//
// Note:
// Unicode whitespace characters are not considered as whitespace characters.
fn parse_active_toolchain(active: &str) -> Result<String> {
    active
        .split_ascii_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Could not determine active toolchain"))
}

pub fn toolchain_path(path: &Path) -> Result<PathBuf> {
    let output = Command::new("rustup")
        .sanitize_environment()
        .current_dir(path)
        .args(["which", "rustc"])
        .logged_output(true)?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    let path = PathBuf::from(stdout);
    // smoelius: `path` should end with `/bin/rustc`.
    path.ancestors()
        .nth(2)
        .map(Into::into)
        .ok_or_else(|| anyhow!("Could not get ancestor"))
}

pub fn is_rustc<T: AsRef<OsStr> + ?Sized>(arg: &T) -> bool {
    Path::new(arg).file_stem() == Some(OsStr::new("rustc"))
}

#[cfg(test)]
mod rustup_test {

    use crate::rustup::{is_rustc, parse_active_toolchain};

    #[test]
    fn rustc_is_rustc() {
        assert!(is_rustc("rustc"));
    }

    #[test]
    fn test_parse_active_toolchain() {
        let outputs = [
            "nightly-aarch64-apple-darwin\ractive because: it's the default toolchain",
            "nightly-x86_64-pc-windows-msvc (default)\r\nactive toolchain",
            "1.85.0-rv64gc-unknown-linux-gnu\nactive because: overridden by '/home/user/rust-with-riscv/rust-toolchain'",
            // allow full width space (\u3000)
            "自定义　rust\nactive because: overridden by '/root/app/rust-toolchain.toml'",
            "私の　rust\r\nactive because: overridden by 'C:\\Users\\watashi\\rust-練習\\rust-toolchain.toml'",
        ];
        let expects = [
            "nightly-aarch64-apple-darwin",
            "nightly-x86_64-pc-windows-msvc",
            "1.85.0-rv64gc-unknown-linux-gnu",
            "自定义　rust",
            "私の　rust",
        ];
        for (output, expect) in outputs.iter().zip(expects.iter()) {
            assert_eq!(parse_active_toolchain(output).unwrap(), *expect);
        }
    }
}

// smoelius: I do not know what the right/best way to parse a toolchain is. `parse_toolchain` does
// so by looking for the architecture.
#[must_use]
pub fn parse_toolchain(toolchain: &str) -> Option<(String, String)> {
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

// smoelius: `ARCHITECTURES` is based on: https://doc.rust-lang.org/rustc/platform-support.html
const ARCHITECTURES: &[&str] = &[
    "aarch64",
    "aarch64_be",
    "amdgcn",
    "arm",
    "arm64_32",
    "arm64e",
    "arm64ec",
    "armeb",
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
    "armv8r",
    "avr",
    "bpfeb",
    "bpfel",
    "csky",
    "hexagon",
    "i386",
    "i586",
    "i686",
    "loongarch32",
    "loongarch64",
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
    "riscv32",
    "riscv32e",
    "riscv32em",
    "riscv32emc",
    "riscv32gc",
    "riscv32i",
    "riscv32im",
    "riscv32ima",
    "riscv32imac",
    "riscv32imafc",
    "riscv32imc",
    "riscv64",
    "riscv64a23",
    "riscv64gc",
    "riscv64imac",
    "s390x",
    "sparc",
    "sparc64",
    "sparcv9",
    "thumbv4t",
    "thumbv5te",
    "thumbv6m",
    "thumbv7a",
    "thumbv7em",
    "thumbv7m",
    "thumbv7neon",
    "thumbv8m.base",
    "thumbv8m.main",
    "wasm32",
    "wasm32v1",
    "wasm64",
    "x86_64",
    "x86_64h",
    "xtensa",
];

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::{ARCHITECTURES, Command};
    use assert_cmd::prelude::*;

    #[test]
    fn architectures_are_current() {
        let output = Command::new("rustc")
            .args(["--print", "target-list"])
            .unwrap();
        let mut architectures = Vec::new();
        for line in std::str::from_utf8(&output.stdout).unwrap().lines() {
            if let Some((architecture, _)) = line.split_once('-') {
                architectures.push(architecture);
            }
        }
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
}
