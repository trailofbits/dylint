use crate::CommandExt;
use anyhow::{Result, bail};
use std::{
    env::consts,
    path::{Path, PathBuf},
};

#[ctor::ctor(unsafe)]
fn init() {
    env_logger::init();
}

pub fn new_template(path: &Path) -> Result<()> {
    crate::packaging::new_template(path)?;
    crate::packaging::use_local_dylint_linting(path)?;
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
