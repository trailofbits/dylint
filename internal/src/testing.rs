use crate::CommandExt;
use anyhow::Result;
use cargo_metadata::MetadataCommand;
use std::{
    env::consts,
    path::{Path, PathBuf},
};

#[ctor::ctor]
fn init() {
    env_logger::init();
}

pub fn new_template(path: &Path) -> Result<()> {
    crate::packaging::new_template(path)?;
    crate::packaging::use_local_packages(path)?;
    Ok(())
}

/// Debug-builds `cargo-dylint` and returns a path to the resulting executable
///
/// Arguments
///
/// - `workspace_root`: path to workspace root
///
/// To run the executable from a test, the likely easiest way is to pass
/// `--path <PATH_TO_LIBRARY_PACKAGE>` or `--lib-path <PATH_TO_DYNAMIC_LIBRARY>`.
pub fn cargo_dylint(workspace_root: impl AsRef<Path>) -> Result<PathBuf> {
    let mut command = crate::cargo::build("`cargo-dylint`").build();
    command
        .current_dir(&workspace_root)
        .args(["--bin", "cargo-dylint"])
        .success()?;

    let metadata = MetadataCommand::new()
        .current_dir(workspace_root.as_ref())
        .no_deps()
        .exec()
        .unwrap();
    let cargo_dylint = metadata
        .target_directory
        .as_std_path()
        .join("debug")
        .join(format!("cargo-dylint{}", consts::EXE_SUFFIX));

    Ok(cargo_dylint)
}
