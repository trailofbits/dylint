use super::{highlight::Highlight, short_id::ShortId, tokenization::tokenize_lines};
use crate::{error::warn, opts};
use anyhow::Result;
use dylint_internal::{
    env,
    git2::{Commit, Diff, DiffHunk, Oid, Patch, Repository},
};
use std::{cmp::min, collections::HashMap, ops::Range, time::Instant};

mod diff;
use diff::{collect_commits, diff_from_commit, patches_from_diff};

const REFACTOR_THRESHOLD: u32 = 3;

#[derive(Eq, PartialEq, Hash)]
pub struct Rewrite {
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub old_tokens: Vec<String>,
    pub new_tokens: Vec<String>,
    pub common_prefix_len: usize,
    pub common_suffix_len: usize,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Rewrite {
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // smoelius: Convert the slices to strings so that even if the alternate format is used, the
        // slices are not printed on multiple lines.
        let before_tokens = format!("{:?}", &self.old_tokens[..self.common_prefix_len]);
        let rewrite_old_tokens = format!(
            "{:?}",
            &self.old_tokens
                [self.common_prefix_len..self.old_tokens.len() - self.common_suffix_len]
        );
        let rewrite_new_tokens = format!(
            "{:?}",
            &self.new_tokens
                [self.common_prefix_len..self.new_tokens.len() - self.common_suffix_len]
        );
        let after_tokens = format!(
            "{:?}",
            &self.old_tokens[self.old_tokens.len() - self.common_suffix_len..]
        );
        f.debug_struct("Rewrite")
            .field("before_tokens", &before_tokens)
            .field(
                "rewrite",
                &format!("{} -> {}", &rewrite_old_tokens, &rewrite_new_tokens),
            )
            .field("after_tokens", &after_tokens)
            .finish()
    }
}

impl Rewrite {
    /// Tries to construct a new [`Rewrite`] from `old_lines` and `new_lines`. Fails if `old_lines`
    /// or `new_lines` cannot be tokenized. Returns `Ok(None)` if the [`Rewrite`] would be an
    /// insertion, i.e., `new_lines`' tokens have a prefix and suffix that, when concatenated, are
    /// `old_lines`' tokens.
    pub fn try_new(old_lines: Vec<&str>, new_lines: Vec<&str>) -> Result<Option<Self>> {
        let old_lines = old_lines
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let new_lines = new_lines
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let old_tokens_borrowed = tokenize_lines(&old_lines)?;
        let new_tokens_borrowed = tokenize_lines(&new_lines)?;
        let old_tokens = old_tokens_borrowed
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let new_tokens = new_tokens_borrowed
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let common_prefix_len = old_tokens
            .iter()
            .zip(new_tokens.iter())
            .take_while(|(x, y)| x == y)
            .count();
        let common_suffix_len = old_tokens
            .iter()
            .rev()
            .zip(new_tokens.iter().rev())
            .take_while(|(x, y)| x == y)
            .count();
        if common_prefix_len + common_suffix_len >= min(old_tokens.len(), new_tokens.len()) {
            Ok(None)
        } else {
            Ok(Some(Self {
                old_lines,
                new_lines,
                old_tokens,
                new_tokens,
                common_prefix_len,
                common_suffix_len,
            }))
        }
    }

    /// Determines whether this [`Rewrite`] would change at least one token in the passed
    /// [`Highlight`]'s highlighted tokens.
    ///
    /// Returns:
    ///
    /// - the "score", i.e., the number of consecutive tokens from this [`Rewrite`] that match the
    ///   [`Highlight`]'s tokens
    /// - the offset into the [`Highlight`]'s tokens where the replacement would start
    pub fn applicability(&self, highlight: &Highlight) -> Option<(usize, usize)> {
        let needle = &self.old_tokens
            [self.common_prefix_len..self.old_tokens.len() - self.common_suffix_len];
        assert!(!needle.is_empty());
        let i = subslice_position(&highlight.tokens, needle)?;
        // smoelius: To be applicable, the needle must change at least one highlighted token.
        if i + needle.len() <= highlight.highlight_start || highlight.highlight_end <= i {
            return None;
        }
        let n_eq_before = self.old_tokens[..self.common_prefix_len]
            .iter()
            .rev()
            .zip(highlight.tokens[..highlight.highlight_start].iter().rev())
            .take_while(|(x, y)| x == y)
            .count();
        let n_eq_after = self.old_tokens[self.old_tokens.len() - self.common_suffix_len..]
            .iter()
            .zip(highlight.tokens[highlight.highlight_end..].iter())
            .take_while(|(x, y)| x == y)
            .count();
        let score = n_eq_before + needle.len() + n_eq_after;
        Some((score, i))
    }
}

// smoelius: Based on: https://stackoverflow.com/a/35907071
fn subslice_position<T: PartialEq>(xs: &[T], ys: &[T]) -> Option<usize> {
    assert!(!ys.is_empty());
    xs.windows(ys.len()).position(|w| w == ys)
}

pub fn collect_rewrites(
    opts: &opts::Dylint,
    old_channel: &str,
    new_oid: Oid,
    repository: &Repository,
) -> Result<HashMap<Rewrite, Oid>> {
    let start = Instant::now();
    let commits = collect_commits(old_channel, new_oid, repository)?;
    let elapsed = start.elapsed();

    eprintln!(
        "Found {} commits in {} seconds",
        commits.len(),
        elapsed.as_secs()
    );

    if env::enabled("DEBUG_COMMITS") {
        display_commits(&commits);
    }

    let start = Instant::now();
    let mut patches_with_oids = Vec::new();
    for commit in commits {
        let oid = commit.id();
        let diff = diff_from_commit(repository, &commit)?;
        let patches = patches_from_diff(&diff)?;
        patches_with_oids.extend(patches.into_iter().map(|patch| (patch, oid)));
    }
    let elapsed = start.elapsed();

    eprintln!(
        "Found {} patches in {} seconds",
        patches_with_oids.len(),
        elapsed.as_secs()
    );

    let start = Instant::now();
    let mut n_insertions = 0;
    let mut n_refactors = 0;
    let mut rewrites = HashMap::new();
    for (patch, oid) in patches_with_oids {
        let rewrites_unflattened =
            rewrites_from_patch(opts, &patch, &mut n_insertions, &mut n_refactors)?;
        for rewrite in rewrites_unflattened {
            rewrites.entry(rewrite).or_insert(oid);
        }
    }
    let elapsed = start.elapsed();

    eprintln!(
        "Extracted {} rewrite rules in {} seconds (discarded {n_insertions} insertions and \
         {n_refactors} refactors)",
        rewrites.len(),
        elapsed.as_secs(),
    );

    Ok(rewrites)
}

// smoelius: You need a `Patch` to get a `DiffHunk`'s lines. So there would be no easy way to write
// a `hunks_from_patch` function. See, for example, `hunk_lines` below.
fn rewrites_from_patch(
    opts: &opts::Dylint,
    patch: &Patch<'_>,
    n_insertions: &mut usize,
    n_refactors: &mut usize,
) -> Result<Vec<Rewrite>> {
    let mut rewrites = Vec::new();
    let n_hunks = patch.num_hunks();
    for hunk_idx in 0..n_hunks {
        let (hunk, line_count) = patch.hunk(hunk_idx)?;
        if (hunk.old_lines() + hunk.new_lines()) as usize != line_count {
            warn(
                opts,
                &format!(
                    "Malformed hunk: old lines ({}) + new lines ({}) != line count ({})",
                    hunk.old_lines(),
                    hunk.new_lines(),
                    line_count
                ),
            );
            continue;
        }
        // smoelius: `hunk.old_lines()` must be non-zero for there to be something to rewrite.
        if hunk.old_lines() == 0 {
            *n_insertions += 1;
            continue;
        }
        if hunk_is_refactor(&hunk) {
            *n_refactors += 1;
            continue;
        }
        let old_lines = hunk_lines(patch, hunk_idx, 0..hunk.old_lines())?;
        let new_lines = hunk_lines(
            patch,
            hunk_idx,
            hunk.old_lines()..hunk.old_lines() + hunk.new_lines(),
        )?;
        if let Some(rewrite) = Rewrite::try_new(old_lines, new_lines)? {
            rewrites.push(rewrite);
        }
    }
    Ok(rewrites)
}

fn hunk_lines<'repo>(
    patch: &'repo Patch<'repo>,
    hunk_idx: usize,
    hunk_lines: Range<u32>,
) -> Result<Vec<&'repo str>> {
    hunk_lines
        .map(|line_of_hunk_u32| -> Result<_> {
            let line_of_hunk = usize::try_from(line_of_hunk_u32)?;
            let diff_line = patch.line_in_hunk(hunk_idx, line_of_hunk)?;
            let line = std::str::from_utf8(diff_line.content())?;
            Ok(line)
        })
        .collect()
}

/// Returns true if, for any hunk in any patch in the diff, both the number of old lines and the
/// number of new lines is three or more.
#[allow(dead_code)]
fn diff_is_refactor(diff: &Diff) -> Result<bool> {
    let stats = diff.stats()?;
    for idx in 0..stats.files_changed() {
        if let Some(patch) = Patch::from_diff(diff, idx)?
            && patch_is_refactor(&patch)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns true if, for any hunk in the patch, both the number of old lines and the number of
/// new lines is three or more.
#[allow(dead_code)]
fn patch_is_refactor(patch: &Patch) -> Result<bool> {
    for hunk_idx in 0..patch.num_hunks() {
        let (hunk, _line_count) = patch.hunk(hunk_idx)?;
        if hunk_is_refactor(&hunk) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns true if both the number of old lines and the number of new lines is three or more.
fn hunk_is_refactor(hunk: &DiffHunk) -> bool {
    hunk.old_lines() >= REFACTOR_THRESHOLD && hunk.new_lines() >= REFACTOR_THRESHOLD
}

fn display_commits(commits: &[Commit]) {
    for commit in commits {
        let short_id = commit.short_id();
        let summary = commit.summary().unwrap_or_default();
        eprintln!("{short_id}: {summary}");
    }
}
