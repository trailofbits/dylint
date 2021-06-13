use crate::toml::{Context, DetailedTomlDependency};
use anyhow::{anyhow, bail, ensure, Result};
use cargo::{
    core::{source::MaybePackage, Dependency, Features, Package, PackageId, Source, SourceId},
    util::{self, Config},
};
use cargo_metadata::{Metadata, MetadataCommand};
use dylint_internal::cargo::SanitizeEnvironment;
use glob::glob;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Library {
    pattern: Option<String>,
    #[serde(flatten)]
    details: DetailedTomlDependency,
}

pub fn dylint_metadata_paths(
    opts: &crate::Dylint,
    metadata: &Metadata,
    object: &Map<String, Value>,
) -> Result<Vec<PathBuf>> {
    if let Some(value) = object.get("dylint") {
        if let Value::Object(object) = value {
            let libraries = object
                .iter()
                .map(|entry| {
                    if entry.0 == "libraries" {
                        let libraries = serde_json::from_value::<Vec<Library>>(entry.1.clone())?;
                        maybe_build_libraries(opts, metadata, &libraries)
                    } else {
                        bail!("Unknown key `{}`", entry.0)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(libraries.into_iter().flatten().collect())
        } else {
            bail!("`dylint` value must be a map")
        }
    } else {
        Ok(vec![])
    }
}

fn maybe_build_libraries(
    opts: &crate::Dylint,
    metadata: &Metadata,
    libraries: &[Library],
) -> Result<Vec<PathBuf>> {
    let config = Config::default()?;

    let paths = libraries
        .iter()
        .map(|library| maybe_build_packages(opts, metadata, &config, library))
        .collect::<Result<Vec<_>>>()?;

    Ok(paths.into_iter().flatten().collect())
}

#[allow(clippy::option_if_let_else)]
fn maybe_build_packages(
    opts: &crate::Dylint,
    metadata: &Metadata,
    config: &Config,
    library: &Library,
) -> Result<Vec<PathBuf>> {
    let dep = dependency(metadata, config, library)?;

    let dependency_root = dependency_root(config, &dep)?;

    let pattern = if let Some(pattern) = &library.pattern {
        dependency_root.join(Path::new(pattern))
    } else {
        dependency_root
    };

    let entries = glob(&pattern.to_string_lossy())?;

    let paths = entries.collect::<std::result::Result<Vec<_>, _>>()?;

    // smoelius: Collecting the package ids before building reveals missing/unparsable `Cargo.toml`
    // files sooner.

    // smoelius: Why are we doing this complicated dance at all? Because we want to leverage Cargo's
    // package cache. But we also want to support git repositories with libraries that use different
    // compiler versions. And we have to work around the fact that "all projects within a workspace
    // are intended to be built with the same version of the compiler"
    // (https://github.com/rust-lang/rustup/issues/1399#issuecomment-383376082).

    let package_root_ids = paths
        .into_iter()
        .filter_map(|path| {
            if path.is_dir() {
                Some(package_id(dep.source_id(), &path).map(|package_id| (path, package_id)))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>>>()?;

    package_root_ids
        .into_iter()
        .map(|(package_root, package_id)| {
            package_library_path(opts, metadata, &package_root, package_id)
        })
        .collect()
}

fn dependency(metadata: &Metadata, config: &Config, library: &Library) -> Result<Dependency> {
    let name_in_toml = "library";

    let mut deps = vec![];
    let root = PathBuf::from(&metadata.workspace_root);
    let source_id = SourceId::for_path(&root)?;
    let mut nested_paths = vec![];
    let mut warnings = vec![];
    let features = Features::new(&[], config, &mut warnings)?;
    let mut cx = Context::new(
        None,
        &mut deps,
        source_id,
        &mut nested_paths,
        config,
        &mut warnings,
        None,
        &root,
        &features,
    );

    let kind = None;

    library.details.to_dependency(name_in_toml, &mut cx, kind)
}

fn dependency_root(config: &Config, dep: &Dependency) -> Result<PathBuf> {
    let source_id = dep.source_id();

    if source_id.is_path() {
        if let Some(path) = source_id.local_path() {
            Ok(path)
        } else {
            bail!("Path source should have a local path: {}", source_id)
        }
    } else if source_id.is_git() {
        git_dependency_root(config, dep)
    } else {
        bail!("Only git and path entries are supported: {}", source_id)
    }
}

#[allow(clippy::default_trait_access)]
fn git_dependency_root(config: &Config, dep: &Dependency) -> Result<PathBuf> {
    let _lock = config.acquire_package_cache_lock();

    let mut source = dep.source_id().load(config, &Default::default())?;

    source.update()?;

    let package_id = sample_package_id(dep, &mut *source)?;

    if let MaybePackage::Ready(package) = source.download(package_id)? {
        git_dependency_root_from_package(config, &*source, &package)
    } else {
        bail!(format!("`{}` is not ready", package_id.name()))
    }
}

fn sample_package_id(dep: &Dependency, source: &mut dyn Source) -> Result<PackageId> {
    let mut package_id: Option<PackageId> = None;

    source.fuzzy_query(dep, &mut |summary| {
        if package_id.is_none() {
            package_id = Some(summary.package_id());
        }
    })?;

    package_id.ok_or_else(|| anyhow!("Found no packages in `{}`", dep.source_id()))
}

fn git_dependency_root_from_package<'a>(
    config: &'a Config,
    source: &(dyn Source + 'a),
    package: &Package,
) -> Result<PathBuf> {
    let package_root = package.root();

    if source.source_id().is_git() {
        let git_path = config.git_path();
        let git_path = config.assert_package_cache_locked(&git_path);
        ensure!(
            package_root.starts_with(git_path.join("checkouts")),
            "Unexpected path: {}",
            package_root.to_string_lossy()
        );
        let n = git_path.components().count() + 3;
        Ok(package_root.components().take(n).collect())
    } else if source.source_id().is_path() {
        unreachable!()
    } else {
        unimplemented!()
    }
}

#[allow(clippy::unwrap_used)]
fn package_id(source_id: SourceId, package_root: &Path) -> Result<PackageId> {
    let metadata = MetadataCommand::new()
        .manifest_path(package_root.join("Cargo.toml"))
        .no_deps()
        .exec()?;

    ensure!(
        metadata.packages.len() <= 1,
        "Library is not its own workspace: {}",
        package_root.to_string_lossy()
    );

    let package = metadata
        .packages
        .first()
        .ok_or_else(|| anyhow!("Found no packages in `{}`", package_root.to_string_lossy()))?;

    assert!(metadata.workspace_root == package.manifest_path.parent().unwrap());

    PackageId::new(&package.name, &package.version.to_string(), source_id)
}

fn package_library_path(
    opts: &crate::Dylint,
    metadata: &Metadata,
    package_root: &Path,
    package_id: PackageId,
) -> Result<PathBuf> {
    let target_dir = target_dir(metadata, package_root, package_id);

    if !opts.no_build {
        dylint_internal::build()
            .sanitize_environment()
            .current_dir(package_root)
            .args(&["--release", "--target-dir", &target_dir.to_string_lossy()])
            .success()?;
    }

    Ok(target_dir.join("release"))
}

fn target_dir(metadata: &Metadata, package_root: &Path, package_id: PackageId) -> PathBuf {
    metadata
        .target_directory
        .join("dylint")
        .join(pkg_dir(package_root, package_id))
        .into()
}

// smoelius: `pkg_dir` and `target_short_hash` are based on functions with the same names in
// https://github.com/rust-lang/cargo/blob/master/src/cargo/core/compiler/context/compilation_files.rs.

fn pkg_dir(package_root: &Path, pkg_id: PackageId) -> String {
    let name = pkg_id.name();
    format!("{}-{}", name, target_short_hash(package_root, pkg_id))
}

const METADATA_VERSION: u8 = 2;

fn target_short_hash(package_root: &Path, pkg_id: PackageId) -> String {
    // smoelius: For now, the package root is the workspace root.
    let hashable = pkg_id.stable_hash(package_root);
    util::short_hash(&(METADATA_VERSION, hashable))
}
