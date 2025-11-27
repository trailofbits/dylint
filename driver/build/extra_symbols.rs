use anyhow::{Context, Result, anyhow, bail};
use dylint_internal::{
    clippy_utils::{Rev, Revs, clippy_repository, parse_as_nightly},
    env,
    rustup::parse_toolchain,
};
use git2::Oid;
use proc_macro2::TokenTree;
use std::{
    fs::{OpenOptions, read_to_string, write},
    io::Write,
    path::PathBuf,
    process::Command,
};
use syn::{Item, ItemMacro, Macro, parse_file};

const SINCE: [u32; 3] = [2025, 5, 6];

const I_YEAR: usize = 0;
const I_MONTH: usize = 1;
const I_DAY: usize = 2;

pub fn build() -> Result<()> {
    let out_dir = env::var(env::OUT_DIR)?;
    let path_buf = PathBuf::from(out_dir).join("extra_symbols.rs");
    // smoelius: `CARGO_MANIFEST_DIR` is the directory of the `dylint_driver` library, not the
    // Dylint driver being built. We cannot easily access the Dylint driver's rust-toolchain file,
    // so we use the `RUSTUP_TOOLCHAIN` environment variable instead.
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    let Some(channel) = clippy_utils_has_extra_symbols(&rustup_toolchain)? else {
        write(&path_buf, "const EXTRA_SYMBOLS: &[&str] = &[];")
            .with_context(|| format!("`write` failed for `{}`", path_buf.display()))?;
        return Ok(());
    };
    let rev = rev_from_channel(&channel)?;
    let sym_rs = clippy_utils_sym_rs(rev.oid)?;
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
        writeln!(file, r#"    {quoted_symbol},"#)
            .with_context(|| format!("`writeln!` failed for `{}`", path_buf.display()))?;
    }
    writeln!(file, "];")
        .with_context(|| format!("`writeln!` failed for `{}`", path_buf.display()))?;
    Ok(())
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
            _ => panic!("unexpected token: {tt:?}"),
        };
        quoted_symbols.push(quoted);
    }
    Ok(quoted_symbols)
}
