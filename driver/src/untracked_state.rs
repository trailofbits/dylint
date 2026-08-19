//! Which lints are registered is an input to compilation that rustc's incremental dependency graph
//! knows nothing about. Writing the variables in [`UNTRACKED_STATE_VARS`] to `sess.env_depinfo`
//! (see `Callbacks::config`) tells Cargo to re-invoke rustc when the input changes, but it does
//! not tell rustc to distrust its cache: `env_depinfo` is written to the dep-info file for the
//! build system to read, and nothing in rustc reads it back. So rustc reloads a cache produced
//! with a different set of registered lints, sees that everything it recorded as an input to its
//! dep-graph nodes is unchanged (it marks them "green"), and ICEs when a query such as
//! `shallow_lint_levels_on` recomputes to a different value anyway. See
//! <https://github.com/trailofbits/dylint/issues/2010>.
//!
//! rustc discards its cache when `Options::dep_tracking_hash` changes, so the fix is to get the
//! state into that hash. `rustc_interface::Config::track_state` looks like the way to do that, but
//! its hasher was removed in <https://github.com/rust-lang/rust/pull/155671>. What remains of the
//! callback can only write to `env_depinfo` and `file_depinfo`, which is the tracking that is
//! already insufficient.
//!
//! `--env-set` reaches the hash instead. Each field of `Options` is declared with a marker saying
//! whether it participates in `dep_tracking_hash`: `[UNTRACKED]` fields do not, `[TRACKED]` fields
//! do and also contribute to the crate hash, and `[TRACKED_NO_CRATE_HASH]` fields do only the
//! former. `--env-set` writes to `logical_env`, which is `[TRACKED]`, so changing its value still
//! causes rustc to discard its cache:
//! <https://github.com/rust-lang/rust/blob/78e7c7b9f9f6a671255ff0fb88b5a6fd6dbc7a58/compiler/rustc_session/src/options.rs#L106-L129>

use anyhow::{Context, Result};
use dylint_internal::env;
use std::{
    fs::read,
    hash::{DefaultHasher, Hasher},
    path::Path,
};

/// The environment variable that carries the hash into `logical_env`.
///
/// Nothing reads the value back. It is set only so that changing it changes `dep_tracking_hash`.
pub(crate) const UNTRACKED_STATE_VAR: &str = "DYLINT_UNTRACKED_STATE";

/// The environment variables whose change should cause lints to be rerun.
///
/// `CARGO_PRIMARY_PACKAGE` and `DYLINT_NO_DEPS` determine whether lints are registered at all,
/// `DYLINT_LIBS` names the libraries that register them, and `DYLINT_METADATA` is the metadata
/// from which those libraries are resolved.
///
/// Read by two consumers that must not drift apart: `sess.env_depinfo`, which tells Cargo when to
/// re-invoke rustc, and [`hash_from_env`], which tells rustc when to discard its incremental
/// cache. A rerun needs both, since re-invoking rustc accomplishes nothing if rustc then reuses
/// results computed under the old values.
pub(crate) const UNTRACKED_STATE_VARS: &[&str] = &[
    env::CARGO_PRIMARY_PACKAGE,
    env::DYLINT_LIBS,
    env::DYLINT_METADATA,
    env::DYLINT_NO_DEPS,
];

/// Reads [`UNTRACKED_STATE_VARS`] from the environment and forwards them to [`hash`].
pub(crate) fn hash_from_env<V: AsRef<Path>>(paths: &[V]) -> Result<Option<String>> {
    let values = UNTRACKED_STATE_VARS
        .iter()
        .map(|var| env::var(var).ok())
        .collect::<Vec<_>>();
    let vars = UNTRACKED_STATE_VARS
        .iter()
        .zip(&values)
        .map(|(&var, value)| (var, value.as_deref()))
        .collect::<Vec<_>>();
    hash(&vars, paths)
}

/// Hashes the state whose change should cause lints to be rerun.
///
/// `vars` are the values of [`UNTRACKED_STATE_VARS`]. They are passed in rather than read from the
/// environment so that this function is a function of its arguments alone.
///
/// Returns `None` if no libraries are loaded, in which case Dylint registers no lints and there is
/// no untracked state to account for.
pub(crate) fn hash<V: AsRef<Path>>(
    vars: &[(&str, Option<&str>)],
    paths: &[V],
) -> Result<Option<String>> {
    if paths.is_empty() {
        return Ok(None);
    }

    // `DefaultHasher` is not guaranteed to produce the same values across Rust versions, but it
    // does not need to. Two runs are compared only if they used the same driver, because the
    // toolchain determines both the target directory and which driver is built.
    let mut hasher = DefaultHasher::new();

    // Distinguish hashes computed by different versions of this function.
    hasher.write(b"dylint-untracked-state-v1");

    for &(var, value) in vars {
        hasher.write(var.as_bytes());
        match value {
            Some(value) => {
                hasher.write(b"=");
                hasher.write(value.as_bytes());
            }
            // Distinguish an unset variable from one set to the empty string.
            None => hasher.write(b"\0"),
        }
    }

    // `DYLINT_LIBS` names the libraries but says nothing about their contents, and
    // `library_filename` derives a library's name from just its lint name and toolchain. So a
    // rebuilt library overwrites the same path and registers a different set of lints without
    // changing any of the variables above. Hash the libraries themselves to catch that.
    //
    // The contents are a proxy for what actually matters, which is the set of lint names the
    // libraries register. That set cannot be known here: obtaining it means registering the lints,
    // which requires a `Session`. But the `Session` is built from the command line on which this
    // hash is passed, so the hash must be computed first. Candidate proxies form a scale of
    // sensitivity: the paths alone, then each library's length and mtime, then its full contents.
    // Prefer the more sensitive end, since an unnecessary change to the hash costs only a needless
    // recompilation.
    let mut paths = paths.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    paths.sort_unstable();
    for path in paths {
        let contents =
            read(path).with_context(|| format!("could not read `{}`", path.to_string_lossy()))?;
        hasher.write(&contents);
    }

    Ok(Some(format!("{:016x}", hasher.finish())))
}
