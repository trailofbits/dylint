use anyhow::{Context, Result, ensure};
use dylint_internal::{env, parse_path_filename};
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeMap, BTreeSet},
    env::split_paths,
    fs::read_dir,
    path::{Path, PathBuf},
};

mod maybe_library;
pub use maybe_library::MaybeLibrary;

pub type ToolchainMap = BTreeMap<String, BTreeSet<PathBuf>>;

#[allow(clippy::redundant_pub_crate)]
pub(crate) type NameToolchainMap = BTreeMap<String, LazyToolchainMap>;

#[allow(clippy::redundant_pub_crate)]
pub(crate) type LazyToolchainMap = BTreeMap<String, BTreeSet<MaybeLibrary>>;

#[cfg_attr(not(__library_packages), allow(dead_code))]
struct Inner<'opts> {
    opts: &'opts crate::opts::Dylint,
    name_toolchain_map: OnceCell<NameToolchainMap>,
}

pub struct Lazy<'opts> {
    inner: Inner<'opts>,
}

impl<'opts> Lazy<'opts> {
    #[must_use]
    pub const fn new(opts: &'opts crate::opts::Dylint) -> Self {
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

                #[cfg(__library_packages)]
                {
                    let library_packages = if self.inner.opts.git_or_path() {
                        crate::library_packages::from_opts(self.inner.opts)?
                    } else {
                        let mut library_packages =
                            crate::library_packages::from_workspace_metadata(self.inner.opts)?;
                        let library_packages_other =
                            crate::library_packages::from_dylint_toml(self.inner.opts)?;
                        ensure!(
                            library_packages.is_empty() || library_packages_other.is_empty(),
                            "`workspace.metadata.dylint.libraries` cannot appear in both \
                             Cargo.toml and dylint.toml"
                        );
                        library_packages.extend(library_packages_other);
                        library_packages
                    };

                    for package in library_packages {
                        name_toolchain_map
                            .entry(package.lib_name.clone())
                            .or_default()
                            .entry(package.toolchain.clone())
                            .or_default()
                            .insert(MaybeLibrary::from(package));
                    }
                }

                // smoelius: If `--git` or `--path` was passed, then do not look for libraries by
                // other means.
                if !self.inner.opts.git_or_path() {
                    let dylint_library_paths = dylint_library_paths()?;

                    for path in dylint_library_paths {
                        for entry in dylint_libraries_in(&path)? {
                            let (name, toolchain, path) = entry?;
                            name_toolchain_map
                                .entry(name)
                                .or_default()
                                .entry(toolchain)
                                .or_default()
                                .insert(MaybeLibrary::from(path));
                        }
                    }
                }

                Ok(name_toolchain_map)
            })
    }
}

fn dylint_library_paths() -> Result<Vec<PathBuf>> {
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
            paths.push(path);
        }
    }

    Ok(paths)
}

fn dylint_libraries_in(
    path: &Path,
) -> Result<impl Iterator<Item = Result<(String, String, PathBuf)>>> {
    let iter = read_dir(path)
        .with_context(|| format!("`read_dir` failed for `{}`", path.to_string_lossy()))?;
    let path_buf = path.to_path_buf();
    let mut libraries = Vec::new();
    for entry in iter {
        let entry = entry
            .with_context(|| format!("`read_dir` failed for `{}`", path_buf.to_string_lossy()))?;
        let entry_path = entry.path();
        if let Some((lib_name, toolchain)) = parse_path_filename(&entry_path) {
            libraries.push(Ok((lib_name, toolchain, entry_path)));
        }
    }
    Ok(libraries.into_iter())
}
