use crate::Dylint;
use anyhow::{anyhow, bail, Context, Result};
use dylint_internal::{find_and_replace, packaging::new_template, rustup::SanitizeEnvironment};
use heck::{ToKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use if_chain::if_chain;
use std::{
    convert::TryFrom,
    fs::{copy, create_dir_all},
    path::Path,
};
use tempfile::tempdir;
use walkdir::WalkDir;

#[cfg(unix)]
mod bisect;

mod backup;
use backup::Backup;

mod clippy_utils;
use clippy_utils::{channel, clippy_utils_version_from_rust_version};

mod revs;
use revs::Revs;

pub fn new_package(opts: &Dylint, path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Could not determine library name from {:?}", path))?;

    let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

    new_template(tempdir.path())?;

    // smoelius: Isolation is now the default.
    if !opts.isolate {
        find_and_replace(
            &tempdir.path().join("Cargo.toml"),
            &[r#"s/\r?\n\[workspace\]\r?\n//"#],
        )?;
    }

    // smoelius: So is allowing unused extern crates.
    find_and_replace(
        &tempdir.path().join("src").join("lib.rs"),
        &[r#"s/(?m)^.. (#!\[warn\(unused_extern_crates\)\])$/${1}/"#],
    )?;

    fill_in(&name, tempdir.path(), path)?;

    Ok(())
}

fn fill_in(name: &str, from: &Path, to: &Path) -> Result<()> {
    let lower_snake_case = name.to_snake_case();
    let upper_snake_case = name.to_shouty_snake_case();
    let kebab_case = name.to_kebab_case();
    let camel_case = name.to_upper_camel_case();

    for entry in WalkDir::new(from) {
        let entry = entry?;
        let abs_path = entry.path();
        let rel_path = abs_path.strip_prefix(from)?;

        if !abs_path.is_file() {
            continue;
        }

        find_and_replace(
            &from.join(rel_path),
            &[&format!(r#"s/\bfill_me_in\b/{}/g"#, lower_snake_case)],
        )?;
        find_and_replace(
            &from.join(rel_path),
            &[&format!(r#"s/\bFILL_ME_IN\b/{}/g"#, upper_snake_case)],
        )?;
        find_and_replace(
            &from.join(rel_path),
            &[&format!(r#"s/\bfill-me-in\b/{}/g"#, kebab_case)],
        )?;
        find_and_replace(
            &from.join(rel_path),
            &[&format!(r#"s/\bFillMeIn\b/{}/g"#, camel_case)],
        )?;

        let from_path = from.join(rel_path);
        let to_path = to.join(rel_path);
        let parent = to_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent directory"))?;
        create_dir_all(parent).with_context(|| {
            format!("`create_dir_all` failed for `{}`", parent.to_string_lossy())
        })?;
        copy(&from_path, &to_path).with_context(|| {
            format!(
                "Could not copy `{}` to `{}`",
                from_path.to_string_lossy(),
                to_path.to_string_lossy()
            )
        })?;
    }

    Ok(())
}

pub fn upgrade_package(opts: &Dylint, path: &Path) -> Result<()> {
    let rev = {
        let revs = Revs::new()?;
        let mut iter = revs.iter()?;
        match &opts.rust_version {
            Some(rust_version) => {
                let clippy_utils_version = clippy_utils_version_from_rust_version(rust_version)?;
                iter.find(|result| {
                    result
                        .as_ref()
                        .map_or(true, |rev| rev.version == clippy_utils_version)
                })
                .unwrap_or_else(|| {
                    Err(anyhow!(
                        "Could not find `clippy_utils` version `{}`",
                        clippy_utils_version
                    ))
                })?
            }
            None => iter.next().unwrap_or_else(|| {
                Err(anyhow!("Could not determine latest `clippy_utils` version"))
            })?,
        }
    };

    let old_channel = channel(path)?;

    let should_find_and_replace = if_chain! {
        if !opts.allow_downgrade;
        if let Some(new_nightly) = parse_as_nightly(&rev.channel);
        if let Some(old_nightly) = parse_as_nightly(&old_channel);
        if new_nightly < old_nightly;
        then {
            if !opts.bisect {
                bail!(
                    "Refusing to downgrade toolchain from `{}` to `{}`. \
                    Use `--allow-downgrade` to override.",
                    old_channel,
                    rev.channel
                );
            }
            false
        } else {
            true
        }
    };

    let cargo_toml_path = path.join("Cargo.toml");
    let rust_toolchain_path = path.join("rust-toolchain");

    let mut cargo_toml_backup =
        Backup::new(&cargo_toml_path).with_context(|| "Could not backup `Cargo.toml`")?;
    let mut rust_toolchain_backup =
        Backup::new(&rust_toolchain_path).with_context(|| "Could not backup `rust-toolchain`")?;

    if should_find_and_replace {
        find_and_replace(
            &cargo_toml_path,
            &[&format!(
                r#"s/(?m)^(clippy_utils\b.*)\b(rev|tag) = "[^"]*"/${{1}}rev = "{}"/"#,
                rev.rev,
            )],
        )?;

        find_and_replace(
            &rust_toolchain_path,
            &[&format!(
                r#"s/(?m)^channel = "[^"]*"/channel = "{}"/"#,
                rev.channel,
            )],
        )?;
    }

    #[cfg(unix)]
    if opts.bisect {
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Could not get file name"))?;
        let description = format!("`{}`", file_name.to_string_lossy());

        dylint_internal::cargo::update(&description, opts.quiet)
            .sanitize_environment()
            .current_dir(path)
            .success()?;

        if dylint_internal::cargo::build(&description, opts.quiet)
            .sanitize_environment()
            .current_dir(path)
            .args(&["--tests"])
            .success()
            .is_err()
        {
            let new_nightly = parse_as_nightly(&rev.channel).ok_or_else(|| {
                anyhow!("Could not not parse channel `{}` as nightly", rev.channel)
            })?;

            let start = format!(
                "{:04}-{:02}-{:02}",
                new_nightly[0], new_nightly[1], new_nightly[2]
            );

            bisect::bisect(opts, path, &start)?;
        }
    }

    rust_toolchain_backup
        .disable()
        .with_context(|| "Could not disable `Cargo.toml` backup")?;
    cargo_toml_backup
        .disable()
        .with_context(|| "Could not disable `rust-toolchain` backup")?;

    Ok(())
}

fn parse_as_nightly(channel: &str) -> Option<[u32; 3]> {
    channel.strip_prefix("nightly-").and_then(parse_date)
}

fn parse_date(date_str: &str) -> Option<[u32; 3]> {
    date_str
        .split('-')
        .map(str::parse::<u32>)
        .map(Result::ok)
        .collect::<Option<Vec<_>>>()
        .map(<[u32; 3]>::try_from)
        .and_then(Result::ok)
}
