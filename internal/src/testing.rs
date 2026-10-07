use crate::{CommandExt, rustup::SanitizeEnvironment};
use anyhow::{Result, bail};
use std::{
    env::consts,
    fs, io,
    path::{Path, PathBuf},
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
pub(crate) fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
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
