use anyhow::{Result, anyhow, bail};
use chrono::{LocalResult, TimeZone, Utc};
use dylint_internal::{
    clippy_utils::parse_as_nightly,
    git2::{Commit, Diff, DiffOptions, Oid, Patch, Repository, Time},
};
use std::ffi::OsStr;

/// Starting with `oid`, works backwards to find all of the commits no earlier than `old_channel`.
pub(super) fn collect_commits<'repo>(
    old_channel: &str,
    new_oid: Oid,
    repository: &'repo Repository,
) -> Result<Vec<Commit<'repo>>> {
    let earliest = channel_to_time(old_channel)?;
    let mut revwalk = repository.revwalk()?;
    revwalk.push(new_oid)?;

    let mut commits = Vec::new();
    for result in revwalk {
        let oid = result?;
        let commit = repository.find_commit(oid)?;
        if commit.time() < earliest {
            break;
        }
        commits.push(commit);
    }
    Ok(commits)
}

fn channel_to_time(channel: &str) -> Result<Time> {
    let [year, month, day] = parse_as_nightly(channel)
        .ok_or_else(|| anyhow!("Channel has unexpected format: {channel}"))?;
    let year_i32 = i32::try_from(year)?;
    let LocalResult::Single(date) = Utc.with_ymd_and_hms(year_i32, month, day, 0, 0, 0) else {
        bail!("Could not construct `DateTime` from channel `{channel}`");
    };
    Ok(Time::new(date.timestamp(), 0))
}

pub(super) fn diff_from_commit<'repo>(
    repository: &'repo Repository,
    new_commit: &Commit,
) -> Result<Diff<'repo>> {
    let old_commit = new_commit.parent(0)?;

    let old_tree = old_commit.tree()?;
    let new_tree = new_commit.tree()?;

    // smoelius: Revisit this. We may want to consider additional context.
    let mut opts = DiffOptions::default();
    opts.context_lines(0);

    let diff = repository.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))?;

    Ok(diff)
}

#[cfg_attr(
    dylint_lib = "misleading_variable_name",
    allow(misleading_variable_name)
)]
pub(super) fn patches_from_diff<'repo>(diff: &Diff<'repo>) -> Result<Vec<Patch<'repo>>> {
    let stats = diff.stats()?;
    let n = stats.files_changed();
    (0..n)
        .map(|idx| -> Result<_> {
            let patch = Patch::from_diff(diff, idx)?;
            // smoelius: Only return patches for Rust source files.
            #[allow(clippy::nonminimal_bool)]
            if !patch
                .as_ref()
                .and_then(|patch| patch.delta().old_file().path())
                .is_some_and(|path| path.extension() == Some(OsStr::new("rs")))
            {
                return Ok(None);
            }
            Ok(patch)
        })
        .filter_map(Result::transpose)
        .collect()
}
