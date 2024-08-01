use crate::{error::warn, opts};
use anyhow::{anyhow, bail, ensure, Result};
use cargo::{
    core::{Dependency, Package as CargoPackage},
    sources::source::{MaybePackage, QueryKind, Source},
};
pub use cargo::{
    core::{PackageId, SourceId},
    util::{cache_lock::CacheLockMode, GlobalContext},
};
use cargo_metadata::Metadata;
use std::path::PathBuf;

pub use cargo_util_schemas::manifest::TomlDetailedDependency;

mod toml;

pub fn dependency_source_id_and_root(
    opts: &opts::Dylint,
    metadata: &Metadata,
    gctx: &GlobalContext,
    details: &TomlDetailedDependency,
) -> Result<(SourceId, PathBuf)> {
    let name_in_toml = "library";

    let mut deps = vec![];
    let root = PathBuf::from(&metadata.workspace_root);
    let source_id = SourceId::for_path(&root)?;
    let mut warnings = vec![];
    let mut cx = toml::ManifestContext::new(&mut deps, source_id, gctx, &mut warnings, None, &root);

    let kind = None;

    let dep = toml::detailed_dep_to_dependency(details, name_in_toml, &mut cx, kind)?;

    if !warnings.is_empty() {
        warn(opts, &warnings.join("\n"));
    }

    let source_id = dep.source_id();

    let root = dependency_root(gctx, &dep)?;

    Ok((source_id, root))
}

fn dependency_root(gctx: &GlobalContext, dep: &Dependency) -> Result<PathBuf> {
    let source_id = dep.source_id();

    if source_id.is_path() {
        if let Some(path) = source_id.local_path() {
            Ok(path)
        } else {
            bail!("Path source should have a local path: {}", source_id)
        }
    } else if source_id.is_git() {
        git_dependency_root(gctx, dep)
    } else {
        bail!("Only git and path entries are supported: {}", source_id)
    }
}

fn git_dependency_root(gctx: &GlobalContext, dep: &Dependency) -> Result<PathBuf> {
    let _lock = gctx.acquire_package_cache_lock(CacheLockMode::DownloadExclusive)?;

    #[allow(clippy::default_trait_access)]
    let mut source = dep.source_id().load(gctx, &Default::default())?;

    let package_id = sample_package_id(dep, &mut *source)?;

    if let MaybePackage::Ready(package) = source.download(package_id)? {
        git_dependency_root_from_package(gctx, &*source, &package)
    } else {
        bail!(format!("`{}` is not ready", package_id.name()))
    }
}

#[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
#[cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]
fn sample_package_id(dep: &Dependency, source: &mut dyn Source) -> Result<PackageId> {
    let mut package_id: Option<PackageId> = None;

    while {
        let poll = source.query(dep, QueryKind::Alternatives, &mut |summary| {
            if package_id.is_none() {
                package_id = Some(summary.package_id());
            }
        })?;
        if poll.is_pending() {
            source.block_until_ready()?;
            package_id.is_none()
        } else {
            false
        }
    } {}

    package_id.ok_or_else(|| anyhow!("Found no packages in `{}`", dep.source_id()))
}

fn git_dependency_root_from_package<'a>(
    gctx: &'a GlobalContext,
    source: &(dyn Source + 'a),
    package: &CargoPackage,
) -> Result<PathBuf> {
    let package_root = package.root();

    if source.source_id().is_git() {
        let git_path = gctx.git_path();
        let git_path =
            gctx.assert_package_cache_locked(CacheLockMode::DownloadExclusive, &git_path);
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
