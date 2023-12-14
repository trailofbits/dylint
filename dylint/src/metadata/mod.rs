use crate::error::warn;
use anyhow::{anyhow, bail, ensure, Context, Result};
use cargo_metadata::{Error, Metadata, MetadataCommand, Package as MetadataPackage};
use dylint_internal::{env, library_filename, rustup::SanitizeEnvironment};
use glob::glob;
use if_chain::if_chain;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::path::{Path, PathBuf};

// smoelius: If both `__metadata_cargo` and `__metadata_cli` are enabled, assume the user built with
// `--features=metadata-cli` and forgot `--no-default-features`.
#[cfg(all(feature = "__metadata_cargo", not(feature = "__metadata_cli")))]
#[path = "cargo/mod.rs"]
mod impl_;

#[cfg(feature = "__metadata_cli")]
#[path = "cli/mod.rs"]
mod impl_;

use impl_::{dependency_source_id_and_root, Config, DetailedTomlDependency, PackageId, SourceId};

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Debug)]
pub struct Package {
    metadata: &'static Metadata,
    pub root: PathBuf,
    pub id: PackageId,
    pub lib_name: String,
    pub toolchain: String,
}

impl Eq for Package {}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        (&self.root, &self.id, &self.lib_name, &self.toolchain)
            == (&other.root, &other.id, &other.lib_name, &other.toolchain)
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.root, &self.id, &self.lib_name, &self.toolchain).cmp(&(
            &other.root,
            &other.id,
            &other.lib_name,
            &other.toolchain,
        ))
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Package {
    pub fn target_directory(&self) -> PathBuf {
        self.metadata
            .target_directory
            .join("dylint/libraries")
            .join(&self.toolchain)
            .into_std_path_buf()
    }

    pub fn path(&self) -> PathBuf {
        self.target_directory()
            .join("release")
            .join(library_filename(&self.lib_name, &self.toolchain))
    }
}

#[derive(Debug, Deserialize)]
struct Library {
    pattern: Option<String>,
    #[serde(flatten)]
    details: DetailedTomlDependency,
}

pub fn workspace_metadata_packages(opts: &crate::Dylint) -> Result<Vec<Package>> {
    if_chain! {
        if let Some(metadata) = cargo_metadata(opts)?;
        if let Some(object) = dylint_metadata(opts)?;
        then {
            dylint_metadata_packages(opts, metadata, object)
        } else {
            Ok(vec![])
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn dylint_metadata(opts: &crate::Dylint) -> Result<Option<&'static Object>> {
    if_chain! {
        if let Some(metadata) = cargo_metadata(opts)?;
        if let serde_json::Value::Object(object) = &metadata.workspace_metadata;
        if let Some(value) = object.get("dylint");
        then {
            if let serde_json::Value::Object(subobject) = value {
                Ok(Some(subobject))
            } else {
                bail!("`dylint` value must be a map")
            }
        } else {
            Ok(None)
        }
    }
}

static CARGO_METADATA: OnceCell<Option<Metadata>> = OnceCell::new();

fn cargo_metadata(opts: &crate::Dylint) -> Result<Option<&'static Metadata>> {
    CARGO_METADATA
        .get_or_try_init(|| {
            if opts.no_metadata {
                return Ok(None);
            }

            let mut command = MetadataCommand::new();

            if let Some(path) = &opts.manifest_path {
                command.manifest_path(path);
            }

            match command.exec() {
                Ok(metadata) => Ok(Some(metadata)),
                Err(err) => {
                    if opts.manifest_path.is_none() {
                        if_chain! {
                            if let Error::CargoMetadata { stderr } = err;
                            if let Some(line) = stderr.lines().next();
                            if !line.starts_with("error: could not find `Cargo.toml`");
                            then {
                                warn(opts, line.strip_prefix("error: ").unwrap_or(line));
                            }
                        }
                        Ok(None)
                    } else {
                        Err(err.into())
                    }
                }
            }
        })
        .map(Option::as_ref)
}

fn dylint_metadata_packages(
    opts: &crate::Dylint,
    metadata: &'static Metadata,
    object: &Object,
) -> Result<Vec<Package>> {
    let libraries = object
        .iter()
        .map(|(key, value)| {
            if key == "libraries" {
                let libraries = serde_json::from_value::<Vec<Library>>(value.clone())?;
                library_packages(opts, metadata, &libraries)
            } else {
                bail!("Unknown key `{}`", key)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(libraries.into_iter().flatten().collect())
}

fn library_packages(
    opts: &crate::Dylint,
    metadata: &'static Metadata,
    libraries: &[Library],
) -> Result<Vec<Package>> {
    let config = Config::default()?;

    let packages = libraries
        .iter()
        .map(|library| library_package(opts, metadata, &config, library))
        .collect::<Result<Vec<_>>>()
        .with_context(|| "Could not build metadata entries")?;

    Ok(packages.into_iter().flatten().collect())
}

fn library_package(
    opts: &crate::Dylint,
    metadata: &'static Metadata,
    config: &Config,
    library: &Library,
) -> Result<Vec<Package>> {
    let details = detailed_toml_dependency(library)?;

    // smoelius: The dependency root cannot be canonicalized here. It could contain a `glob` pattern
    // (e.g., `*`), because Dylint allows `path` entries to contain `glob` patterns.
    let (source_id, dependency_root) =
        dependency_source_id_and_root(opts, metadata, config, details)?;

    let pattern = if let Some(pattern) = &library.pattern {
        dependency_root.join(pattern)
    } else {
        #[allow(clippy::redundant_clone)]
        dependency_root.clone()
    };

    let entries = glob(&pattern.to_string_lossy())?;

    let paths = entries
        .map(|entry| {
            entry.map_err(Into::into).and_then(|path| {
                // smoelius: Because `dependency_root` might not be absolute, `path` might not be
                // absolute. So `path` must be normalized.
                let path_buf = cargo_util::paths::normalize_path(&path);
                if let Some(pattern) = &library.pattern {
                    // smoelius: Use `cargo_util::paths::normalize_path` instead of `canonicalize`
                    // so as not to "taint" the path with a path prefix on Windows.
                    //
                    // This problem keeps coming up. For example, it recently came up in:
                    // https://github.com/trailofbits/dylint/pull/944
                    let dependency_root = cargo_util::paths::normalize_path(&dependency_root);
                    ensure!(
                        path_buf.starts_with(&dependency_root),
                        "Pattern `{pattern}` could refer to `{}`, which is outside of `{}`",
                        path_buf.to_string_lossy(),
                        dependency_root.to_string_lossy()
                    );
                }
                Ok(path_buf)
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    ensure!(
        !paths.is_empty(),
        "No paths matched `{}`",
        pattern.to_string_lossy()
    );

    // smoelius: Collecting the package ids before building reveals missing/unparsable `Cargo.toml`
    // files sooner.

    // smoelius: Why are we doing this complicated dance at all? Because we want to leverage Cargo's
    // download cache. But we also want to support git repositories with libraries that use
    // different compiler versions. And we have to work around the fact that "all projects within a
    // workspace are intended to be built with the same version of the compiler"
    // (https://github.com/rust-lang/rustup/issues/1399#issuecomment-383376082).

    // smoelius: Experiments suggest that a considerable amount of Dylint's start up time is spent
    // in the following "loop," and a considerable (though not necessarily dominant) fraction of
    // that is spent in `active_toolchain`.
    let packages = paths
        .into_iter()
        .map(|path| {
            if path.is_dir() {
                // smoelius: Ignore subdirectories that do not contain packages.
                let Ok(package) = package_with_root(&path) else {
                    return Ok(None);
                };
                // smoelius: When `__metadata_cli` is enabled, `source_id`'s type is `String`.
                #[allow(clippy::clone_on_copy)]
                let package_id = package_id(&package, source_id.clone())?;
                let lib_name = package_library_name(&package)?;
                let toolchain = dylint_internal::rustup::active_toolchain(&path)?;
                Ok(Some(Package {
                    metadata,
                    root: path,
                    id: package_id,
                    lib_name,
                    toolchain,
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(packages.into_iter().flatten().collect())
}

fn detailed_toml_dependency(library: &Library) -> Result<&DetailedTomlDependency> {
    let mut unused_keys = library.details.unused_keys();
    #[allow(clippy::format_collect)]
    if !unused_keys.is_empty() {
        unused_keys.sort_unstable();
        bail!(
            "Unknown library keys:{}",
            unused_keys
                .iter()
                .map(|name| format!("\n    {name}"))
                .collect::<String>()
        );
    }

    Ok(&library.details)
}

fn package_with_root(package_root: &Path) -> Result<MetadataPackage> {
    // smoelius: For the long term, we should investigate having a "cache" that maps paths to
    // `cargo_metadata::Metadata`.
    let metadata = MetadataCommand::new()
        .current_dir(package_root)
        .no_deps()
        .exec()?;

    dylint_internal::cargo::package_with_root(&metadata, package_root)
}

fn package_id(package: &MetadataPackage, source_id: SourceId) -> Result<PackageId> {
    PackageId::new(package.name.clone(), package.version.clone(), source_id)
}

pub fn package_library_name(package: &MetadataPackage) -> Result<String> {
    package
        .targets
        .iter()
        .find_map(|target| {
            if target.kind.iter().any(|kind| kind == "cdylib") {
                Some(target.name.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow!(
                "Could not find `cdylib` target for package `{}`",
                package.id
            )
        })
}

pub fn build_library(opts: &crate::Dylint, package: &Package) -> Result<PathBuf> {
    let target_dir = package.target_directory();

    let path = package.path();

    if !opts.no_build {
        // smoelius: Clear `RUSTFLAGS` so that changes to it do not cause workspace metadata entries
        // to be rebuilt.
        dylint_internal::cargo::build(&format!("workspace metadata entry `{}`", package.id.name()))
            .quiet(opts.quiet)
            .build()
            .sanitize_environment()
            .env_remove(env::RUSTFLAGS)
            .current_dir(&package.root)
            .args(["--release", "--target-dir", &target_dir.to_string_lossy()])
            .success()?;

        let exists = path
            .try_exists()
            .with_context(|| format!("Could not determine whether {path:?} exists"))?;

        ensure!(exists, "Could not find {path:?} despite successful build");
    }

    Ok(path)
}

// smoelius: `pkg_dir` and `target_short_hash` are based on functions with the same names in
// https://github.com/rust-lang/cargo/blob/master/src/cargo/core/compiler/context/compilation_files.rs

#[cfg(any())]
mod disabled {
    fn pkg_dir(package_root: &Path, pkg_id: PackageId) -> String {
        let name = pkg_id.name();
        format!("{}-{}", name, target_short_hash(package_root, pkg_id))
    }

    const METADATA_VERSION: u8 = 2;

    fn target_short_hash(package_root: &Path, pkg_id: PackageId) -> String {
        // smoelius: For now, the package root is the workspace root.
        let hashable = pkg_id.stable_hash(package_root);
        cargo::util::short_hash(&(METADATA_VERSION, hashable))
    }
}
