use crate::Dylint;
use anyhow::{anyhow, Result};
use dylint_internal::find_and_replace;
use git2::Repository;
use heck::*;
use lazy_static::lazy_static;
use semver::Version;
use std::{
    fs::{copy, create_dir_all, read_to_string},
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use walkdir::WalkDir;

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

    let checked_out = tempdir()?;
    let filtered = tempdir()?;

    dylint_internal::clone(DYLINT_TEMPLATE_URL, "master", checked_out.path())?;

    if opts.isolate {
        dylint_internal::packaging::isolate(checked_out.path())?;
    }

    filter(&name, checked_out.path(), filtered.path())?;

    rename(&name, filtered.path(), path)?;

    Ok(())
}

fn filter(name: &str, from: &Path, to: &Path) -> Result<()> {
    let lower_snake_case = name.to_snake_case();

    for path in &*PATHS {
        let to_path = if path == &Path::new("src").join("fill_me_in.rs") {
            to.join("src").join(&lower_snake_case).with_extension("rs")
        } else {
            to.join(path)
        };
        to_path.parent().map(create_dir_all).transpose()?;
        copy(from.join(path), to_path)?;
    }

    Ok(())
}

fn rename(name: &str, from: &Path, to: &Path) -> Result<()> {
    let lower_snake_case = name.to_snake_case();
    let upper_snake_case = name.to_shouty_snake_case();
    let kebab_case = name.to_kebab_case();
    let camel_case = name.to_camel_case();

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

        to.join(rel_path).parent().map(create_dir_all).transpose()?;
        copy(from.join(rel_path), to.join(rel_path))?;
    }

    Ok(())
}

pub fn upgrade_package(opts: &Dylint, path: &Path) -> Result<()> {
    let tempdir = tempdir()?;

    let refname = match &opts.rust_version {
        Some(rust_version) => format!("rust-{}", rust_version),
        None => "master".to_owned(),
    };

    let repository = dylint_internal::clone(RUST_CLIPPY_URL, &refname, tempdir.path())?;

    let tag = match &opts.rust_version {
        Some(rust_version) => format!("rust-{}", rust_version),
        None => {
            let version = latest_rust_version(&repository)?;
            format!("rust-{}", version.to_string())
        }
    };

    dylint_internal::checkout(&repository, &tag)?;

    let channel_line = channel_line(tempdir.path())?;

    find_and_replace(
        &path.join("Cargo.toml"),
        &[&format!(
            r#"s/(?m)^(clippy_utils\b.*)\btag = "[^"]*"/${{1}}tag = "{}"/"#,
            tag,
        )],
    )?;

    find_and_replace(
        &path.join("rust-toolchain"),
        &[&format!(r#"s/(?m)^channel = .*/{}/"#, channel_line,)],
    )?;

    Ok(())
}

fn latest_rust_version(repository: &Repository) -> Result<Version> {
    let tags = repository.tag_names(Some("rust-*"))?;
    let mut rust_versions = tags
        .iter()
        .map(|s| s.map(|s| s.strip_prefix("rust-")).flatten())
        .flatten()
        .map(Version::parse)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rust_versions.sort();
    rust_versions
        .pop()
        .ok_or_else(|| anyhow!("Could not determine latest `clippy_utils` version"))
}

fn channel_line(path: &Path) -> Result<String> {
    let file = read_to_string(path.join("rust-toolchain"))?;
    file.lines()
        .find(|line| line.starts_with("channel = "))
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("Could not determine Rust toolchain channel"))
}
