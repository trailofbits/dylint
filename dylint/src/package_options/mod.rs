use crate::Dylint;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{Date, NaiveDate, TimeZone, Utc};
use dylint_internal::{find_and_replace, rustup::SanitizeEnvironment};
use heck::{ToKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use if_chain::if_chain;
use lazy_static::lazy_static;
use std::{
    fs::{copy, create_dir_all, rename},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, NamedTempFile};
use walkdir::WalkDir;

#[cfg(unix)]
mod bisect;

mod clippy_utils;
use clippy_utils::{channel, clippy_utils_version_from_rust_version};

mod revs;
use revs::Revs;

struct Backup {
    path: PathBuf,
    tempfile: NamedTempFile,
    disabled: bool,
}

impl Backup {
    pub fn new(dir: &Path, entry: &str) -> Result<Self> {
        let path = dir.join(entry);
        let tempfile =
            NamedTempFile::new_in(dir).with_context(|| "Could not create named temp file")?;
        copy(&path, tempfile.path())
            .with_context(|| format!("Could not copy {:?} to {:?}", path, tempfile.path()))?;
        Ok(Self {
            path,
            tempfile,
            disabled: false,
        })
    }

    pub fn disable(&mut self) {
        self.disabled = true;
    }
}

impl Drop for Backup {
    fn drop(&mut self) {
        if !self.disabled {
            rename(self.tempfile.path(), &self.path).unwrap_or_default();
        }
    }
}

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";

lazy_static! {
    static ref PATHS: [PathBuf; 7] = [
        PathBuf::from(".cargo").join("config.toml"),
        PathBuf::from(".gitignore"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("rust-toolchain"),
        PathBuf::from("src").join("lib.rs"),
        PathBuf::from("ui").join("main.rs"),
        PathBuf::from("ui").join("main.stderr"),
    ];
}

pub fn new_package(opts: &Dylint, path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Could not determine library name from {:?}", path))?;

    let checked_out = tempdir().with_context(|| "`tempdir` failed")?;
    let filtered = tempdir().with_context(|| "`tempdir` failed")?;

    dylint_internal::clone(DYLINT_TEMPLATE_URL, "master", checked_out.path())?;

    if opts.isolate {
        dylint_internal::packaging::isolate(checked_out.path())?;
    }

    filter(&name, checked_out.path(), filtered.path())?;

    fill_in(&name, filtered.path(), path)?;

    Ok(())
}

fn filter(_name: &str, from: &Path, to: &Path) -> Result<()> {
    for path in &*PATHS {
        let from_path = from.join(path);
        let to_path = to.join(path);
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

    let mut cargo_toml_backup = Backup::new(path, "Cargo.toml")?;
    let mut rust_toolchain_backup = Backup::new(path, "rust-toolchain")?;

    if should_find_and_replace {
        find_and_replace(
            &path.join("Cargo.toml"),
            &[&format!(
                r#"s/(?m)^(clippy_utils\b.*)\b(rev|tag) = "[^"]*"/${{1}}rev = "{}"/"#,
                rev.rev,
            )],
        )?;

        find_and_replace(
            &path.join("rust-toolchain"),
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

            let start = new_nightly.format("%Y-%m-%d").to_string();

            bisect::bisect(opts, path, &start)?;
        }
    }

    rust_toolchain_backup.disable();
    cargo_toml_backup.disable();

    Ok(())
}

fn parse_as_nightly(channel: &str) -> Option<Date<Utc>> {
    channel
        .strip_prefix("nightly-")
        .and_then(utc_from_manifest_date)
}

// smoelius: `utc_from_manifest_date` is from
// https://github.com/rust-lang/rustup/blob/master/src/dist/dist.rs
fn utc_from_manifest_date(date_str: &str) -> Option<Date<Utc>> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .map(|date| Utc.from_utc_date(&date))
}
