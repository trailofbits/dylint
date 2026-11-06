use crate::{CommandExt, find_and_replace, rustup::SanitizeEnvironment};
use anyhow::{Result, bail};
use cargo_metadata::{Dependency, semver::Version};
use std::{
    env::consts,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};
use tempfile::tempdir;

#[ctor::ctor(unsafe)]
fn init() {
    env_logger::init();
}

pub fn new_template(path: &Path) -> Result<()> {
    copy_dir(&TEMPLATE_PATH, path).map_err(Into::into)
}

static TEMPLATE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let tempdir = tempdir().unwrap();

    new_template_inner(tempdir.path()).unwrap();

    crate::cargo::build("testing template")
        .build()
        .sanitize_environment()
        .current_dir(&tempdir)
        .success()
        .unwrap();

    tempdir.keep()
});

fn new_template_inner(path: &Path) -> Result<()> {
    crate::packaging::new_template(path)?;
    crate::packaging::use_local_dylint_linting(path)?;
    Ok(())
}

// smoelius: `copy_dir` is based on:
// https://github.com/rust-lang/rustup/blob/31a98d3e5ab8990af12d8023fa1dd7038ccc2205/src/utils/raw.rs#L234-L248
// But the version here requires `dest` to exist.
pub fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
    for entry in src.read_dir()? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let src = entry.path();
        let dest = dest.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir(&dest)?;
            copy_dir(&src, &dest)?;
        } else {
            fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

pub fn new_filled_in_template(cargo_dylint: &Path, path: &Path, name: &str) -> Result<PathBuf> {
    let path_buf = path.join(name);

    Command::new(cargo_dylint)
        .args(["dylint", "new", &path_buf.to_string_lossy(), "--isolate"])
        .success()?;

    check_dylint_dependencies(&path_buf)?;

    crate::packaging::use_local_dylint_linting(&path_buf)?;

    allow_unused_extern_crates(&path_buf)?;

    Ok(path_buf)
}

/// Debug-builds `cargo-dylint` and returns a path to the resulting executable
///
/// To run the executable from a test, the likely easiest way is to pass
/// `--path <PATH_TO_LIBRARY_PACKAGE>` or `--lib-path <PATH_TO_DYNAMIC_LIBRARY>`.
pub fn cargo_dylint() -> Result<PathBuf> {
    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    let Some(parent) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() else {
        bail!("Could not get parent directory");
    };

    crate::cargo::build("`cargo-dylint`")
        .build()
        .current_dir(parent)
        .args(["--bin", "cargo-dylint"])
        .success()?;

    let metadata = crate::cargo::metadata(parent).unwrap();
    let cargo_dylint = metadata
        .target_directory
        .as_std_path()
        .join("debug")
        .join(format!("cargo-dylint{}", consts::EXE_SUFFIX));

    Ok(cargo_dylint)
}

fn check_dylint_dependencies(dir: &Path) -> Result<()> {
    let metadata = crate::cargo::metadata(dir)?;
    for package in metadata.packages {
        for Dependency { name: dep, req, .. } in &package.dependencies {
            if dep.starts_with("dylint") {
                if package.name.as_str() == "filled_in" {
                    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                    assert!(req.matches(&version));
                } else {
                    assert_eq!("^".to_owned() + env!("CARGO_PKG_VERSION"), req.to_string());
                }
            }
        }
    }
    Ok(())
}

// smoelius: `allow_unused_extern_crates` is essentially the inverse of the `find_and_replace` in
// `dylint::package_options::new_package`. That is, the line that was uncommented gets recommented.
// This is a hack, but it affects only one test, and only its ui.
fn allow_unused_extern_crates(path: &Path) -> Result<()> {
    find_and_replace(
        &path.join("src/lib.rs"),
        r"(?m)^(#!\[warn\(unused_extern_crates\)\])$",
        "// ${1}",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn new_template_works() {
        let tempdir = tempdir().unwrap();
        new_template(tempdir.path()).unwrap();
    }
}
