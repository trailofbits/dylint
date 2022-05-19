use anyhow::{ensure, Context, Result};
use dylint_internal::{env, parse_path_filename};
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, BTreeSet},
    env::split_paths,
    fs::read_dir,
    path::{Path, PathBuf},
};

pub type ToolchainMap = BTreeMap<String, BTreeSet<PathBuf>>;

#[allow(clippy::redundant_pub_crate)]
pub(crate) type NameToolchainMap = BTreeMap<String, ToolchainMap>;

#[cfg_attr(not(feature = "metadata"), allow(dead_code))]
struct Inner<'opts> {
    opts: &'opts crate::Dylint,
    name_toolchain_map: OnceCell<NameToolchainMap>,
}

pub struct Lazy<'opts> {
    inner: Inner<'opts>,
}

impl<'opts> Lazy<'opts> {
    #[must_use]
    pub const fn new(opts: &'opts crate::Dylint) -> Self {
        Self {
            inner: Inner {
                opts,
                name_toolchain_map: OnceCell::new(),
            },
        }
    }

    pub fn get_or_try_init(&self) -> Result<&NameToolchainMap> {
        self.inner
            .name_toolchain_map
            .get_or_try_init(|| -> Result<_> {
                let mut name_toolchain_map = NameToolchainMap::new();

                let dylint_library_paths = dylint_library_paths()?;
                #[cfg(feature = "metadata")]
                let workspace_metadata_paths =
                    crate::metadata::workspace_metadata_paths(self.inner.opts)?;
                #[cfg(not(feature = "metadata"))]
                let workspace_metadata_paths = vec![];

                for (path, require_existence) in
                    dylint_library_paths.iter().chain(&workspace_metadata_paths)
                {
                    if !require_existence && !path.exists() {
                        continue;
                    }
                    for entry in dylint_libraries_in(path)? {
                        let (name, toolchain, path) = entry?;
                        name_toolchain_map
                            .entry(name)
                            .or_insert_with(Default::default)
                            .entry(toolchain)
                            .or_insert_with(Default::default)
                            .insert(path);
                    }
                }

                Ok(name_toolchain_map)
            })
    }
}

fn dylint_library_paths() -> Result<Vec<(PathBuf, bool)>> {
    let mut paths = Vec::new();

    if let Ok(val) = env::var(env::DYLINT_LIBRARY_PATH) {
        for path in split_paths(&val) {
            ensure!(
                path.is_absolute(),
                "DYLINT_LIBRARY_PATH contains `{}`, which is not absolute",
                path.to_string_lossy()
            );
            ensure!(
                path.is_dir(),
                "DYLINT_LIBRARY_PATH contains `{}`, which is not a directory",
                path.to_string_lossy()
            );
            paths.push((path, true));
        }
    }

    Ok(paths)
}

#[allow(clippy::option_if_let_else)]
fn dylint_libraries_in(
    path: &Path,
) -> Result<impl Iterator<Item = Result<(String, String, PathBuf)>>> {
    let iter = read_dir(path)
        .with_context(|| format!("`read_dir` failed for `{}`", path.to_string_lossy()))?;
    let path = path.to_path_buf();
    Ok(iter
        .map(move |entry| -> Result<Option<(String, String, PathBuf)>> {
            let entry = entry
                .with_context(|| format!("`read_dir` failed for `{}`", path.to_string_lossy()))?;
            let path = entry.path();

            Ok(parse_path_filename(&path).map(|(lib_name, toolchain)| (lib_name, toolchain, path)))
        })
        .filter_map(Result::transpose))
}
