use crate::Dylint;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{Date, NaiveDate, TimeZone, Utc};
use dylint_internal::{find_and_replace, rustup::SanitizeEnvironment};
use git2::Repository;
use heck::{ToKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use if_chain::if_chain;
use lazy_static::lazy_static;
use semver::Version;
use std::{
    fs::{copy, create_dir_all, read_to_string, rename},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, NamedTempFile};
use walkdir::WalkDir;

#[cfg(unix)]
mod bisect;

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

const RUST_CLIPPY_URL: &str = "https://github.com/rust-lang/rust-clippy";

lazy_static! {
    static ref PATHS: [PathBuf; 8] = [
        PathBuf::from(".cargo").join("config.toml"),
        PathBuf::from(".gitignore"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("rust-toolchain"),
        PathBuf::from("src").join("fill_me_in.rs"),
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

fn filter(name: &str, from: &Path, to: &Path) -> Result<()> {
    let lower_snake_case = name.to_snake_case();

    for path in &*PATHS {
        let from_path = from.join(path);
        let to_path = if path == &Path::new("src").join("fill_me_in.rs") {
            to.join("src").join(&lower_snake_case).with_extension("rs")
        } else {
            to.join(path)
        };
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
    let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

    let refname = match &opts.rust_version {
        Some(rust_version) => format!("rust-{}", rust_version),
        None => "master".to_owned(),
    };

    let repository = dylint_internal::clone(RUST_CLIPPY_URL, &refname, tempdir.path())?;

    let tag = match &opts.rust_version {
        Some(rust_version) => format!("rust-{}", rust_version),
        None => {
            let version = latest_rust_version(&repository)?;
            format!("rust-{}", version)
        }
    };

    dylint_internal::checkout(&repository, &tag)?;

    let new_channel = channel(tempdir.path())?;

    let old_channel = channel(path)?;

    let should_find_and_replace = if_chain! {
        if !opts.force;
        if let Some(new_nightly) = parse_as_nightly(&new_channel);
        if let Some(old_nightly) = parse_as_nightly(&old_channel);
        if new_nightly < old_nightly;
        then {
            if !opts.bisect {
                bail!(
                    "Refusing to downgrade toolchain from `{}` to `{}`. Use `--force` to override.",
                    old_channel,
                    new_channel
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
                r#"s/(?m)^(clippy_utils\b.*)\btag = "[^"]*"/${{1}}tag = "{}"/"#,
                tag,
            )],
        )?;

        find_and_replace(
            &path.join("rust-toolchain"),
            &[&format!(
                r#"s/(?m)^channel = "[^"]*"/channel = "{}"/"#,
                new_channel,
            )],
        )?;
    }

    #[cfg(unix)]
    if opts.bisect {
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Could not get file name"))?;
        let description = format!("`{}`", file_name.to_string_lossy());

        dylint_internal::update(&description, opts.quiet)
            .sanitize_environment()
            .current_dir(path)
            .success()?;

        if dylint_internal::build(&description, opts.quiet)
            .sanitize_environment()
            .current_dir(path)
            .args(&["--tests"])
            .success()
            .is_err()
        {
            let new_nightly = parse_as_nightly(&new_channel).ok_or_else(|| {
                anyhow!("Could not not parse channel `{}` as nightly", new_channel)
            })?;

            let start = new_nightly.format("%Y-%m-%d").to_string();

            bisect::bisect(opts, path, &start)?;
        }
    }

    rust_toolchain_backup.disable();
    cargo_toml_backup.disable();

    Ok(())
}

fn latest_rust_version(repository: &Repository) -> Result<Version> {
    let tags = repository.tag_names(Some("rust-*"))?;
    let mut rust_versions = tags
        .iter()
        .filter_map(|s| s.and_then(|s| s.strip_prefix("rust-")))
        .map(Version::parse)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rust_versions.sort();
    rust_versions
        .pop()
        .ok_or_else(|| anyhow!("Could not determine latest `clippy_utils` version"))
}

fn channel(path: &Path) -> Result<String> {
    let rust_toolchain = path.join("rust-toolchain");
    let file = read_to_string(&rust_toolchain).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            rust_toolchain.to_string_lossy(),
        )
    })?;
    file.lines()
        .find_map(|line| line.strip_prefix(r#"channel = ""#))
        .and_then(|line| line.strip_suffix('"'))
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine Rust toolchain channel"))
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
