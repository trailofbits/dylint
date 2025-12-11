use crate::{error::warn, opts};
use anyhow::{Context, Result, anyhow, bail, ensure};
use cargo_metadata::{Error, Metadata, MetadataCommand, Package as MetadataPackage, TargetKind};
use cargo_util_schemas::manifest::{StringOrVec, TomlDetailedDependency};
use dylint_internal::{CommandExt, config, env, library_filename, rustup::SanitizeEnvironment};
use glob::glob;
use once_cell::sync::OnceCell;
use serde::{Deserialize, de::IntoDeserializer};
use std::path::{Path, PathBuf};

#[cfg(feature = "__cargo_cli")]
#[path = "cargo_cli/mod.rs"]
mod impl_;

use impl_::{GlobalContext, PackageId, SourceId, dependency_source_id_and_root};

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Debug)]
pub struct Package {
    /// [`Metadata`] of the workspace bing linted
    ///
    /// Caching a reference to the [`Metadata`] here ensures that it existed when the package was
    /// created.
    metadata: &'static Metadata,
    /// Path to the package's directory
    pub root: PathBuf,
    /// The package's [`PackageId`], which includes the package's name, version, and source id
    pub id: PackageId,
    /// Name of the `cdylib` the package builds
    pub lib_name: String,
    /// Toolchain the package uses
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
    pattern: Option<StringOrVec>,
    #[serde(flatten)]
    details: TomlDetailedDependency,
}

pub fn from_opts(opts: &opts::Dylint) -> Result<Vec<Package>> {
    let lib_sel = opts.library_selection();

    let maybe_metadata = cargo_metadata(opts)?;

    let metadata = maybe_metadata.ok_or_else(|| anyhow!("Could not read cargo metadata"))?;

    ensure!(
        lib_sel.paths.len() <= 1,
        "At most one library package can be named with `--path`"
    );

    let path = if let Some(path) = lib_sel.paths.first() {
        let canonical_path = dunce::canonicalize(path)
            .with_context(|| format!("Could not canonicalize {path:?}"))?;
        Some(canonical_path.to_string_lossy().to_string())
    } else {
        None
    };

    let toml: toml::map::Map<_, _> = vec![
        to_map_entry("path", path.as_ref()),
        to_map_entry("git", lib_sel.git.as_ref()),
        to_map_entry("branch", lib_sel.branch.as_ref()),
        to_map_entry("tag", lib_sel.tag.as_ref()),
        to_map_entry("rev", lib_sel.rev.as_ref()),
    ]
    .into_iter()
    .flatten()
    .collect();

    let details = TomlDetailedDependency::deserialize(toml.into_deserializer())?;

    let library = Library {
        details,
        pattern: lib_sel
            .pattern
            .as_ref()
            .map(|pattern| StringOrVec(vec![pattern.clone()])),
    };

    library_packages(opts, metadata, &[library])
}

fn to_map_entry(key: &str, value: Option<&String>) -> Option<(String, toml::Value)> {
    value
        .cloned()
        .map(|s| (String::from(key), toml::Value::from(s)))
}

pub fn from_workspace_metadata(opts: &opts::Dylint) -> Result<Vec<Package>> {
    if let Some(metadata) = cargo_metadata(opts)?
        && let Some(object) = dylint_metadata(opts)?
    {
        library_packages_from_dylint_metadata(opts, metadata, object)
    } else {
        Ok(vec![])
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn dylint_metadata(opts: &opts::Dylint) -> Result<Option<&'static Object>> {
    if let Some(metadata) = cargo_metadata(opts)?
        && let serde_json::Value::Object(object) = &metadata.workspace_metadata
        && let Some(value) = object.get("dylint")
    {
        if let serde_json::Value::Object(subobject) = value {
            Ok(Some(subobject))
        } else {
            bail!("`dylint` value must be a map")
        }
    } else {
        Ok(None)
    }
}

static CARGO_METADATA: OnceCell<Option<Metadata>> = OnceCell::new();

fn cargo_metadata(opts: &opts::Dylint) -> Result<Option<&'static Metadata>> {
    CARGO_METADATA
        .get_or_try_init(|| {
            let lib_sel = opts.library_selection();

            if lib_sel.no_metadata {
                return Ok(None);
            }

            let mut command = MetadataCommand::new();

            if let Some(path) = &lib_sel.manifest_path {
                command.manifest_path(path);
            }

            match command.exec() {
                Ok(metadata) => Ok(Some(metadata)),
                Err(err) => {
                    if lib_sel.manifest_path.is_none() {
                        if let Error::CargoMetadata { stderr } = err
                            && let Some(line) = stderr.lines().next()
                            && !line.starts_with("error: could not find `Cargo.toml`")
                        {
                            warn(opts, line.strip_prefix("error: ").unwrap_or(line));
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

fn library_packages_from_dylint_metadata(
    opts: &opts::Dylint,
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
                bail!("Unknown key `{key}`")
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(libraries.into_iter().flatten().collect())
}

pub fn from_dylint_toml(opts: &opts::Dylint) -> Result<Vec<Package>> {
    if let Some(metadata) = cargo_metadata(opts)? {
        let _ = config::try_init_with_metadata(metadata)?;
        if let Some(table) = config::get() {
            library_packages_from_dylint_toml(opts, metadata, table)
        } else {
            Ok(vec![])
        }
    } else {
        Ok(vec![])
    }
}

fn library_packages_from_dylint_toml(
    opts: &opts::Dylint,
    metadata: &'static Metadata,
    table: &toml::Table,
) -> Result<Vec<Package>> {
    let Some(config_metadata) = table
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("metadata"))
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("dylint"))
    else {
        return Ok(Vec::new());
    };

    let Some(table) = config_metadata.as_table() else {
        bail!("`dylint` value must be a table");
    };

    let libraries = table
        .iter()
        .map(|(key, value)| {
            if key == "libraries" {
                let libraries = Vec::<Library>::deserialize(value.clone().into_deserializer())?;
                library_packages(opts, metadata, &libraries)
            } else {
                bail!("Unknown key `{key}`")
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(libraries.into_iter().flatten().collect())
}

fn library_packages(
    opts: &opts::Dylint,
    metadata: &'static Metadata,
    libraries: &[Library],
) -> Result<Vec<Package>> {
    let gctx = GlobalContext::default()?;

    let packages = libraries
        .iter()
        .map(|library| library_package(opts, metadata, &gctx, library))
        .collect::<Result<Vec<_>>>()
        .with_context(|| "Could not build metadata entries")?;

    Ok(packages.into_iter().flatten().collect())
}

fn library_package(
    opts: &opts::Dylint,
    metadata: &'static Metadata,
    gctx: &GlobalContext,
    library: &Library,
) -> Result<Vec<Package>> {
    let details = toml_detailed_dependency(library)?;

    // smoelius: The dependency root cannot be canonicalized here. It could contain a `glob` pattern
    // (e.g., `*`), because Dylint allows `path` entries to contain `glob` patterns.
    let (source_id, dependency_root) =
        dependency_source_id_and_root(opts, metadata, gctx, details)?;

    let patterns = if let Some(StringOrVec(patterns)) = &library.pattern {
        patterns.clone()
    } else {
        vec![String::new()]
    };

    let mut paths = Vec::new();
    for pattern in patterns {
        let results = glob(&dependency_root.join(&pattern).to_string_lossy())?;

        let mut matched = false;
        for result in results {
            let path = result?;

            // smoelius: Because `dependency_root` might not be absolute, `path` might not be
            // absolute. So `path` must be normalized.
            let path_buf = cargo_util::paths::normalize_path(&path);

            // smoelius: If `library.pattern` is set, verify the `path` that it matched is in
            // `dependency_root`.
            //
            // Note that even if `library.pattern` is not set, the current loop must still be
            // traversed. Recall, `dependency_root` could be a `glob` pattern. In such a
            // case, the paths that it matches must be pushed onto `paths`.
            if library.pattern.is_some() {
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

            paths.push(path_buf);
            matched = true;
        }

        if !matched {
            if library.pattern.is_some() {
                bail!("No paths matched `{pattern}`");
            }
            bail!(
                "No library packages found in `{}`",
                dependency_root.display()
            );
        }
    }

    // smoelius: Collecting the package ids before building reveals missing/unparsable `Cargo.toml`
    // files sooner.

    // smoelius: Why are we doing this complicated dance at all? Because we want to leverage Cargo's
    // download cache. But we also want to support git repositories with libraries that use
    // different compiler versions. And we have to work around the fact that "all projects within a
    // workspace are intended to be built with the same version of the compiler"
    // (https://github.com/rust-lang/rustup/issues/1399#issuecomment-383376082).

    // smoelius: Experiments suggest that a considerable amount of Dylint's start up time is spent
    // in the following loop, and a considerable (though not necessarily dominant) fraction of that
    // is spent in `active_toolchain`.
    let mut packages = Vec::new();
    for path in paths {
        if !path.is_dir() {
            continue;
        }
        // smoelius: Ignore subdirectories that do not contain packages.
        let package = match package_with_root(&path) {
            Ok(package) => package,
            Err(error) => {
                warn(opts, &error.to_string());
                continue;
            }
        };
        // smoelius: When `__cargo_cli` is enabled, `source_id`'s type is `String`.
        #[allow(clippy::clone_on_copy)]
        let package_id = package_id(&package, source_id.clone());
        let lib_name = package_library_name(&package)?;
        let toolchain = dylint_internal::rustup::active_toolchain(&path)?;
        packages.push(Package {
            metadata,
            root: path,
            id: package_id,
            lib_name,
            toolchain,
        });
    }

    Ok(packages)
}

fn toml_detailed_dependency(library: &Library) -> Result<&TomlDetailedDependency> {
    let mut unused_keys = library
        .details
        ._unused_keys
        .keys()
        .cloned()
        .collect::<Vec<_>>();

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
    // smoelius: Both `cargo_path` and `sanitize_environment` must be called. Without the call to
    // `cargo_path`, `cargo_metadata` will use `$CARGO`. Without the call to `sanitize_environment`,
    // `CARGO`, etc. will be set and the `rustup` proxy will invoke the wrong `cargo`.
    let metadata = MetadataCommand::new()
        .cargo_path("cargo")
        .sanitize_environment()
        .current_dir(package_root)
        .no_deps()
        .exec()?;

    dylint_internal::cargo::package_with_root(&metadata, package_root)
}

fn package_id(package: &MetadataPackage, source_id: SourceId) -> PackageId {
    PackageId::new(
        #[allow(clippy::useless_conversion)]
        package.name.to_string().into(),
        package.version.clone(),
        source_id,
    )
}

pub fn package_library_name(package: &MetadataPackage) -> Result<String> {
    package
        .targets
        .iter()
        .find_map(|target| {
            if target.kind.iter().any(|kind| kind == &TargetKind::CDyLib) {
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

pub fn build_library(opts: &opts::Dylint, package: &Package) -> Result<PathBuf> {
    let target_dir = package.target_directory();

    let path = package.path();

    if !opts.library_selection().no_build {
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
            .with_context(|| format!("Could not determine whether `{}` exists", path.display()))?;

        ensure!(
            exists,
            "Could not find `{}` despite successful build",
            path.display()
        );
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
