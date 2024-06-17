// smoelius: This file is essentially the dependency specific portions of
// https://github.com/rust-lang/cargo/blob/0.80.0/src/cargo/util/toml/mod.rs with adjustments to
// make some things public.
// smoelius: I experimented with creating a reduced Cargo crate that included just this module and
// the things it depends upon. Such a crate could reduce build times and incur less of a maintenance
// burden than this file. However, Cargo's modules appear to be highly interdependent, as can be
// seen by running the following command in the root of the Cargo repository:
//
//    cargo modules generate graph --package cargo --lib --uses
//
// Hence, I think that idea is a dead end.

#![allow(unused_imports)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::if_not_else)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::use_self)]
#![cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
#![cfg_attr(
    dylint_lib = "inconsistent_qualification",
    allow(inconsistent_qualification)
)]
#![cfg_attr(
    dylint_lib = "misleading_variable_name",
    allow(misleading_variable_name)
)]
#![cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]

// smoelius: `manifest::TomlDetailedDependency::unused_keys` does not appear in the original.
impl super::UnusedKeys for manifest::TomlDetailedDependency {
    fn unused_keys(&self) -> Vec<String> {
        self._unused_keys.keys().cloned().collect()
    }
}

// smoelius: `ManifestContext::new` does not appear in the original.
#[allow(clippy::too_many_arguments)]
impl<'a, 'b> ManifestContext<'a, 'b> {
    pub fn new(
        deps: &'a mut Vec<Dependency>,
        source_id: SourceId,
        gctx: &'b GlobalContext,
        warnings: &'a mut Vec<String>,
        platform: Option<Platform>,
        root: &'a Path,
    ) -> Self {
        Self {
            deps,
            source_id,
            gctx,
            warnings,
            platform,
            root,
        }
    }
}

// use annotate_snippets::{Level, Renderer, Snippet};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::{self, FromStr};

// use crate::AlreadyPrintedError;
use anyhow::{anyhow, bail, Context as _};
use cargo_platform::Platform;
use cargo_util::paths::{self, normalize_path};
use cargo_util_schemas::manifest::{self, TomlManifest};
use cargo_util_schemas::manifest::{RustVersion, StringOrBool};
// use itertools::Itertools;
// use lazycell::LazyCell;
// use pathdiff::diff_paths;
// use url::Url;

use crate::core::compiler::{CompileKind, CompileTarget};
use crate::core::dependency::{Artifact, ArtifactTarget, DepKind};
use crate::core::manifest::{ManifestMetadata, TargetSourcePath};
use crate::core::resolver::ResolveBehavior;
use crate::core::FeatureValue::Dep;
use crate::core::{find_workspace_root, resolve_relative_path, CliUnstable, FeatureValue};
use crate::core::{Dependency, Manifest, Package, PackageId, Summary, Target};
use crate::core::{Edition, EitherManifest, Feature, Features, VirtualManifest, Workspace};
use crate::core::{GitReference, PackageIdSpec, SourceId, WorkspaceConfig, WorkspaceRootConfig};
use crate::sources::{CRATES_IO_INDEX, CRATES_IO_REGISTRY};
use crate::util::errors::{CargoResult, ManifestError};
use crate::util::interning::InternedString;
use crate::util::{self, context::ConfigRelativePath, GlobalContext, IntoUrl, OptVersionReq};

#[allow(dead_code)]
pub struct ManifestContext<'a, 'b> {
    deps: &'a mut Vec<Dependency>,
    source_id: SourceId,
    gctx: &'b GlobalContext,
    warnings: &'a mut Vec<String>,
    platform: Option<Platform>,
    root: &'a Path,
}

pub fn detailed_dep_to_dependency<P: ResolveToPath + Clone>(
    orig: &manifest::TomlDetailedDependency<P>,
    name_in_toml: &str,
    manifest_ctx: &mut ManifestContext<'_, '_>,
    kind: Option<DepKind>,
) -> CargoResult<Dependency> {
    if orig.version.is_none() && orig.path.is_none() && orig.git.is_none() {
        anyhow::bail!(
            "dependency ({name_in_toml}) specified without \
                 providing a local path, Git repository, version, or \
                 workspace dependency to use"
        );
    }

    if let Some(version) = &orig.version {
        if version.contains('+') {
            manifest_ctx.warnings.push(format!(
                "version requirement `{}` for dependency `{}` \
                     includes semver metadata which will be ignored, removing the \
                     metadata is recommended to avoid confusion",
                version, name_in_toml
            ));
        }
    }

    if orig.git.is_none() {
        let git_only_keys = [
            (&orig.branch, "branch"),
            (&orig.tag, "tag"),
            (&orig.rev, "rev"),
        ];

        for &(key, key_name) in &git_only_keys {
            if key.is_some() {
                bail!(
                    "key `{}` is ignored for dependency ({}).",
                    key_name,
                    name_in_toml
                );
            }
        }
    }

    // Early detection of potentially misused feature syntax
    // instead of generating a "feature not found" error.
    if let Some(features) = &orig.features {
        for feature in features {
            if feature.contains('/') {
                bail!(
                    "feature `{}` in dependency `{}` is not allowed to contain slashes\n\
                         If you want to enable features of a transitive dependency, \
                         the direct dependency needs to re-export those features from \
                         the `[features]` table.",
                    feature,
                    name_in_toml
                );
            }
            if feature.starts_with("dep:") {
                bail!(
                    "feature `{}` in dependency `{}` is not allowed to use explicit \
                        `dep:` syntax\n\
                         If you want to enable an optional dependency, specify the name \
                         of the optional dependency without the `dep:` prefix, or specify \
                         a feature from the dependency's `[features]` table that enables \
                         the optional dependency.",
                    feature,
                    name_in_toml
                );
            }
        }
    }

    let new_source_id = to_dependency_source_id(orig, name_in_toml, manifest_ctx)?;

    let (pkg_name, explicit_name_in_toml) = match orig.package {
        Some(ref s) => (&s[..], Some(name_in_toml)),
        None => (name_in_toml, None),
    };

    let version = orig.version.as_deref();
    let mut dep = Dependency::parse(pkg_name, version, new_source_id)?;
    dep.set_features(orig.features.iter().flatten())
        .set_default_features(orig.default_features().unwrap_or(true))
        .set_optional(orig.optional.unwrap_or(false))
        .set_platform(manifest_ctx.platform.clone());
    if let Some(registry) = &orig.registry {
        let registry_id = SourceId::alt_registry(manifest_ctx.gctx, registry)?;
        dep.set_registry_id(registry_id);
    }
    if let Some(registry_index) = &orig.registry_index {
        let url = registry_index.into_url()?;
        let registry_id = SourceId::for_registry(&url)?;
        dep.set_registry_id(registry_id);
    }

    if let Some(kind) = kind {
        dep.set_kind(kind);
    }
    if let Some(name_in_toml) = explicit_name_in_toml {
        dep.set_explicit_name_in_toml(name_in_toml);
    }

    if let Some(p) = orig.public {
        dep.set_public(p);
    }

    #[cfg(any())]
    if let (Some(artifact), is_lib, target) = (
        orig.artifact.as_ref(),
        orig.lib.unwrap_or(false),
        orig.target.as_deref(),
    ) {
        if manifest_ctx.gctx.cli_unstable().bindeps {
            let artifact = Artifact::parse(&artifact.0, is_lib, target)?;
            if dep.kind() != DepKind::Build
                && artifact.target() == Some(ArtifactTarget::BuildDependencyAssumeTarget)
            {
                bail!(
                    r#"`target = "target"` in normal- or dev-dependencies has no effect ({})"#,
                    name_in_toml
                );
            }
            dep.set_artifact(artifact)
        } else {
            bail!("`artifact = …` requires `-Z bindeps` ({})", name_in_toml);
        }
    } else if orig.lib.is_some() || orig.target.is_some() {
        for (is_set, specifier) in [
            (orig.lib.is_some(), "lib"),
            (orig.target.is_some(), "target"),
        ] {
            if !is_set {
                continue;
            }
            bail!(
                "'{}' specifier cannot be used without an 'artifact = …' value ({})",
                specifier,
                name_in_toml
            )
        }
    }
    Ok(dep)
}

fn to_dependency_source_id<P: ResolveToPath + Clone>(
    orig: &manifest::TomlDetailedDependency<P>,
    name_in_toml: &str,
    manifest_ctx: &mut ManifestContext<'_, '_>,
) -> CargoResult<SourceId> {
    match (
        orig.git.as_ref(),
        orig.path.as_ref(),
        orig.registry.as_deref(),
        orig.registry_index.as_ref(),
    ) {
        (Some(_git), _, Some(_registry), _) | (Some(_git), _, _, Some(_registry)) => bail!(
            "dependency ({name_in_toml}) specification is ambiguous. \
                 Only one of `git` or `registry` is allowed.",
        ),
        (_, _, Some(_registry), Some(_registry_index)) => bail!(
            "dependency ({name_in_toml}) specification is ambiguous. \
                 Only one of `registry` or `registry-index` is allowed.",
        ),
        (Some(_git), Some(_path), None, None) => {
            bail!(
                "dependency ({name_in_toml}) specification is ambiguous. \
                     Only one of `git` or `path` is allowed.",
            );
        }
        (Some(git), None, None, None) => {
            let n_details = [&orig.branch, &orig.tag, &orig.rev]
                .iter()
                .filter(|d| d.is_some())
                .count();

            if n_details > 1 {
                bail!(
                    "dependency ({name_in_toml}) specification is ambiguous. \
                         Only one of `branch`, `tag` or `rev` is allowed.",
                );
            }

            let reference = orig
                .branch
                .clone()
                .map(GitReference::Branch)
                .or_else(|| orig.tag.clone().map(GitReference::Tag))
                .or_else(|| orig.rev.clone().map(GitReference::Rev))
                .unwrap_or(GitReference::DefaultBranch);
            let loc = git.into_url()?;

            if let Some(fragment) = loc.fragment() {
                let msg = format!(
                    "URL fragment `#{fragment}` in git URL is ignored for dependency ({name_in_toml}). \
                        If you were trying to specify a specific git revision, \
                        use `rev = \"{fragment}\"` in the dependency declaration.",
                );
                manifest_ctx.warnings.push(msg);
            }

            SourceId::for_git(&loc, reference)
        }
        (None, Some(path), _, _) => {
            let path = path.resolve(manifest_ctx.gctx);
            // If the source ID for the package we're parsing is a path
            // source, then we normalize the path here to get rid of
            // components like `..`.
            //
            // The purpose of this is to get a canonical ID for the package
            // that we're depending on to ensure that builds of this package
            // always end up hashing to the same value no matter where it's
            // built from.
            if manifest_ctx.source_id.is_path() {
                let path = manifest_ctx.root.join(path);
                let path = paths::normalize_path(&path);
                SourceId::for_path(&path)
            } else {
                Ok(manifest_ctx.source_id)
            }
        }
        (None, None, Some(registry), None) => SourceId::alt_registry(manifest_ctx.gctx, registry),
        (None, None, None, Some(registry_index)) => {
            let url = registry_index.into_url()?;
            SourceId::for_registry(&url)
        }
        (None, None, None, None) => SourceId::crates_io(manifest_ctx.gctx),
    }
}

pub trait ResolveToPath {
    fn resolve(&self, gctx: &GlobalContext) -> PathBuf;
}

impl ResolveToPath for String {
    fn resolve(&self, _: &GlobalContext) -> PathBuf {
        self.into()
    }
}
