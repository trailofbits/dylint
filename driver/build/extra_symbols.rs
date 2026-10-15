use anyhow::{Context, Result, anyhow, bail};
use dylint_internal::{
    clippy_utils::{
        Rev, Revs, clippy_repository, clippy_repository_path, parse_as_nightly, toolchain_channel,
    },
    env,
    rustup::parse_toolchain,
};
use git2::Oid;
use proc_macro2::TokenTree;
use std::{
    fs::{OpenOptions, read_to_string, write},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use syn::{Item, ItemMacro, Macro, parse_file};

const SINCE: [u32; 3] = [2025, 5, 6];

const I_YEAR: usize = 0;
const I_MONTH: usize = 1;
const I_DAY: usize = 2;

pub fn build() -> Result<()> {
    println!("cargo:rerun-if-env-changed={}", env::DYLINT_CLIPPY_REPO);
    println!("cargo:rerun-if-env-changed={}", env::RUSTUP_TOOLCHAIN);
    let out_dir = env::var(env::OUT_DIR)?;
    let path_buf = PathBuf::from(out_dir).join("extra_symbols.rs");
    // smoelius: `CARGO_MANIFEST_DIR` is the directory of the `dylint_driver` library, not the
    // Dylint driver being built. We cannot easily access the Dylint driver's rust-toolchain
    // file, so we use the `RUSTUP_TOOLCHAIN` environment variable instead.
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    let sym_rs = if let Some(path) = clippy_repository_path()? {
        supplied_clippy_sym_rs(&path, &rustup_toolchain)?
    } else {
        let Some(channel) = clippy_utils_has_extra_symbols(&rustup_toolchain)? else {
            write(&path_buf, "const EXTRA_SYMBOLS: &[&str] = &[];")
                .with_context(|| format!("`write` failed for `{}`", path_buf.display()))?;
            return Ok(());
        };
        let rev = rev_from_channel(&channel)?;
        clippy_utils_sym_rs(rev.oid)?
    };
    let quoted_symbols = quoted_symbols(&sym_rs)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path_buf)
        .with_context(|| format!("Could not open `{}`", path_buf.display()))?;
    writeln!(file, "const EXTRA_SYMBOLS: &[&str] = &[")
        .with_context(|| format!("`writeln!` failed for `{}`", path_buf.display()))?;
    for quoted_symbol in quoted_symbols {
        writeln!(file, "    {quoted_symbol},")
            .with_context(|| format!("`writeln!` failed for `{}`", path_buf.display()))?;
    }
    writeln!(file, "];")
        .with_context(|| format!("`writeln!` failed for `{}`", path_buf.display()))?;
    Ok(())
}

fn supplied_clippy_sym_rs(path: &Path, rustup_toolchain: &str) -> Result<String> {
    // EricLiu2000: Track the directory so adding rust-toolchain also invalidates a build
    // that previously read rust-toolchain.toml, without tracking a nonexistent file.
    println!("cargo:rerun-if-changed={}", path.display());
    let channel = toolchain_channel(path)?;
    let compiler_channel = parse_toolchain(rustup_toolchain)
        .map_or_else(|| rustup_toolchain.to_owned(), |(channel, _)| channel);
    if parse_as_nightly(&compiler_channel).is_some() && compiler_channel != channel {
        bail!(
            "Clippy source toolchain `{channel}` does not match driver toolchain `{compiler_channel}`"
        );
    }
    let sym_path = path.join("clippy_utils/src/sym.rs");
    read_to_string(&sym_path).with_context(|| format!("Could not read `{}`", sym_path.display()))
}

fn clippy_utils_has_extra_symbols(rustup_toolchain: &str) -> Result<Option<String>> {
    let (channel, _) = parse_toolchain(rustup_toolchain)
        .ok_or_else(|| anyhow!("Could not parse toolchain: {rustup_toolchain}"))?;

    if channel == "nightly" {
        return Ok(None);
    }

    let Some([year, month, day]) = parse_as_nightly(&channel) else {
        return Ok(None);
    };

    if (year < SINCE[I_YEAR])
        || (year == SINCE[I_YEAR] && month < SINCE[I_MONTH])
        || (year == SINCE[I_YEAR] && month == SINCE[I_MONTH] && day < SINCE[I_DAY])
    {
        return Ok(None);
    }

    Ok(Some(channel))
}

fn rev_from_channel(channel: &str) -> Result<Rev> {
    let revs = Revs::new(false)?;
    let mut iter = revs.channel_iter()?;
    // smoelius: Stop at the first channel found lexicographically at or earlier than the desired
    // channel.
    match iter.find(|rev| rev.as_ref().map_or(true, |rev| *rev.channel <= *channel)) {
        Some(result) => result,
        None => Err(anyhow!("Could not find revision for channel {channel}")),
    }
}

fn clippy_utils_sym_rs(rev: Oid) -> Result<String> {
    let repository = clippy_repository(false)?;
    let Some(workdir) = repository.workdir() else {
        bail!("Repository has no working directory");
    };

    let mut command = Command::new("git");
    command
        .current_dir(workdir)
        .args(["checkout", &rev.to_string()]);
    let status = command
        .status()
        .with_context(|| format!("command failed: {command:?}"))?;
    assert!(status.success());

    let sym_path = workdir.join("clippy_utils/src/sym.rs");
    read_to_string(&sym_path)
        .with_context(|| format!("`read_to_string` failed for `{}`", sym_path.display()))
}

fn quoted_symbols(sym_rs: &str) -> Result<Vec<String>> {
    let file = parse_file(sym_rs)?;
    let mut stream = None;
    for item in file.items {
        if let Item::Macro(ItemMacro {
            mac: Macro { path, tokens, .. },
            ..
        }) = item
            && path.is_ident("generate")
        {
            stream = Some(tokens);
            break;
        }
    }
    let stream = stream.ok_or_else(|| anyhow!("Could not find `generate!` macro invocation"))?;
    let tokens = stream.into_iter().collect::<Vec<_>>();
    let subslices =
        tokens.split(|tt| matches!(tt, TokenTree::Punct(punct) if punct.as_char() == ','));
    let mut quoted_symbols = Vec::new();
    for subslice in subslices {
        let Some(tt) = subslice.last() else {
            continue;
        };
        let quoted = match tt {
            TokenTree::Ident(ident) => format!(r#""{ident}""#),
            TokenTree::Literal(lit) => lit.to_string(),
            _ => bail!("Unexpected token in Clippy symbols: {tt:?}"),
        };
        quoted_symbols.push(quoted);
    }
    Ok(quoted_symbols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, remove_file, write};
    use tempfile::{TempDir, tempdir};

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn source_snapshot_without_git() {
        let dir = snapshot();
        assert!(!dir.path().join(".git").exists());
        for toolchain in ["nightly-2026-10-14-aarch64-apple-darwin", "custom"] {
            let source = supplied_clippy_sym_rs(dir.path(), toolchain).unwrap();
            assert_eq!(
                &[r#""First""#, r#""second value""#],
                quoted_symbols(&source).unwrap().as_slice()
            );
        }
        assert!(!dir.path().join(".git").exists());
    }

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn source_snapshot_toolchain_mismatch() {
        let dir = snapshot();
        let error = supplied_clippy_sym_rs(dir.path(), "nightly-2026-10-13").unwrap_err();
        assert!(error.to_string().contains("does not match"));
    }

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn source_snapshot_uses_rustup_file_precedence() {
        let dir = snapshot();
        write(
            dir.path().join("rust-toolchain"),
            "[toolchain]\nchannel = \"nightly-2026-10-13\"\n",
        )
        .unwrap();
        assert!(supplied_clippy_sym_rs(dir.path(), "nightly-2026-10-13").is_ok());
        assert!(supplied_clippy_sym_rs(dir.path(), "nightly-2026-10-14").is_err());
    }

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn invalid_snapshot_fails() {
        let dir = snapshot();
        let symbols = dir.path().join("clippy_utils/src/sym.rs");
        for invalid in ["not Rust!", "fn main() {}", "generate! { invalid(), }"] {
            write(&symbols, invalid).unwrap();
            let source = supplied_clippy_sym_rs(dir.path(), "custom").unwrap();
            assert!(quoted_symbols(&source).is_err());
        }
        remove_file(symbols).unwrap();
        assert!(supplied_clippy_sym_rs(dir.path(), "custom").is_err());
        remove_file(dir.path().join("rust-toolchain.toml")).unwrap();
        assert!(supplied_clippy_sym_rs(dir.path(), "custom").is_err());
    }

    fn snapshot() -> TempDir {
        let dir = tempdir().unwrap();
        create_dir_all(dir.path().join("clippy_utils/src")).unwrap();
        write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"nightly-2026-10-14\"\n",
        )
        .unwrap();
        write(
            dir.path().join("clippy_utils/src/sym.rs"),
            r#"generate! { First, Second: "second value", }"#,
        )
        .unwrap();
        dir
    }
}
