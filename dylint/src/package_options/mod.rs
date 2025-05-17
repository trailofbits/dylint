use crate::opts;
use anyhow::{Context, Result, anyhow, bail};
use dylint_internal::{
    clippy_utils::{
        clippy_utils_version_from_rust_version, set_clippy_utils_dependency_revision,
        set_toolchain_channel, toolchain_channel,
    },
    find_and_replace,
    packaging::new_template,
};
use heck::{ToKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use if_chain::if_chain;
use rewriter::Backup;
use std::{
    env::current_dir,
    fs::{copy, create_dir_all},
    path::Path,
};
use tempfile::tempdir;
use walkdir::WalkDir;

mod auto_correct;
use auto_correct::auto_correct;

mod common;
use common::parse_as_nightly;

mod revs;
use revs::Revs;

pub fn new_package(_opts: &opts::Dylint, new_opts: &opts::New) -> Result<()> {
    let path = Path::new(&new_opts.path);

    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Could not determine library name from {:?}", path))?;

    let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

    new_template(tempdir.path())?;

    // smoelius: Isolation is now the default.
    if !new_opts.isolate {
        find_and_replace(
            &tempdir.path().join("Cargo.toml"),
            r"\r?\n\[workspace\]\r?\n",
            "",
        )?;
    }

    // smoelius: So is allowing unused extern crates.
    find_and_replace(
        &tempdir.path().join("src/lib.rs"),
        r"(?m)^.. (#!\[warn\(unused_extern_crates\)\])$",
        "${1}",
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

        find_and_replace(&from.join(rel_path), r"\bfill_me_in\b", &lower_snake_case)?;
        find_and_replace(&from.join(rel_path), r"\bFILL_ME_IN\b", &upper_snake_case)?;
        find_and_replace(&from.join(rel_path), r"\bfill-me-in\b", &kebab_case)?;
        find_and_replace(&from.join(rel_path), r"\bFillMeIn\b", &camel_case)?;

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

pub fn upgrade_package(opts: &opts::Dylint, upgrade_opts: &opts::Upgrade) -> Result<()> {
    let current_dir = current_dir().with_context(|| "Could not get current directory")?;

    let path = match &upgrade_opts.path {
        Some(path_str) => Path::new(path_str),
        None => &current_dir,
    };

    // Instantiate Revs once
    let revs = Revs::new(opts.quiet)?;

    let rev = match &upgrade_opts.rust_version {
        Some(rust_version) => {
            // Find the specific version using the new find_version
            let clippy_utils_version = clippy_utils_version_from_rust_version(rust_version)?;
            let found_version = revs.find_version(&clippy_utils_version)?;
            found_version.ok_or_else(|| {
                anyhow!(
                    "Could not find `clippy_utils` version `{}`",
                    clippy_utils_version
                )
            })?
        }
        None => {
            // Find the latest version using the new find_latest_rev
            revs.find_latest_rev()?
        }
    };

    let old_channel = toolchain_channel(path)?;

    if_chain! {
        if !upgrade_opts.allow_downgrade;
        if let Some(new_nightly) = parse_as_nightly(&rev.channel);
        if let Some(old_nightly) = parse_as_nightly(&old_channel);
        if new_nightly < old_nightly;
        then {
            bail!(
                "Refusing to downgrade toolchain from `{}` to `{}`. \
                Use `--allow-downgrade` to override.",
                old_channel,
                rev.channel
            );
        }
    }

    let rust_toolchain_path = path.join("rust-toolchain");
    let cargo_toml_path = path.join("Cargo.toml");

    let mut rust_toolchain_backup =
        Backup::new(rust_toolchain_path).with_context(|| "Could not backup `rust-toolchain`")?;
    let mut cargo_toml_backup =
        Backup::new(cargo_toml_path).with_context(|| "Could not backup `Cargo.toml`")?;

    set_toolchain_channel(path, &rev.channel)?;
    set_clippy_utils_dependency_revision(path, &rev.oid.to_string())?;

    if upgrade_opts.auto_correct {
        auto_correct(opts, upgrade_opts, &old_channel, rev.oid)?;
    }

    cargo_toml_backup
        .disable()
        .with_context(|| "Could not disable `Cargo.toml` backup")?;
    rust_toolchain_backup
        .disable()
        .with_context(|| "Could not disable `rust-toolchain` backup")?;

    Ok(())
}
