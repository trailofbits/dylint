#![allow(clippy::unwrap_used)]

use super::Backup;
use crate::opts;
use anyhow::{Context, Result};
use dylint_internal::{clippy_utils::clippy_repository, git2::Oid};
use rewriter::{LineColumn, Rewriter, Span, interface::Span as _};
use std::{
    collections::{BTreeMap, HashMap},
    env::current_dir,
    fs::{read_to_string, write},
    ops::Range,
    path::Path,
};

mod tokenization;
use tokenization::tokenize_lines;

mod highlight;
use highlight::{Highlight, collect_highlights};

mod rewrite;
use rewrite::{Rewrite, collect_rewrites};

mod short_id;
use short_id::ShortId;

/// Information about the application of a [`Rewrite`]
#[derive(Debug)]
struct ReplacementSource<'rewrite> {
    /// Score returned by [`Rewrite::applicability`]
    score: usize,
    /// Span of the text to be replaced
    span: Span,
    /// Commit from which the rewrite originated
    oid: Oid,
    /// The rewrite itself
    rewrite: &'rewrite Rewrite,
}

// smoelius: For each replacement, we store only the `Rewrite` with the best score.
type ReplacementSourceMap<'rewrite> = BTreeMap<String, ReplacementSource<'rewrite>>;

enum Reason<'rewrite> {
    None,
    Multiple(usize, ReplacementSourceMap<'rewrite>),
}

pub fn auto_correct(
    opts: &opts::Dylint,
    upgrade_opts: &opts::Upgrade,
    old_channel: &str,
    new_oid: Oid,
) -> Result<()> {
    let mut backups = BTreeMap::new();

    auto_correct_revertible(opts, upgrade_opts, old_channel, new_oid, &mut backups)?;

    for (file_name, mut backup) in backups {
        backup
            .disable()
            .with_context(|| format!("Could not disable `{file_name}` backup"))?;
    }

    Ok(())
}

#[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
#[allow(clippy::module_name_repetitions, clippy::similar_names)]
pub fn auto_correct_revertible(
    opts: &opts::Dylint,
    upgrade_opts: &opts::Upgrade,
    old_channel: &str,
    new_oid: Oid,
    backups: &mut BTreeMap<String, Backup>,
) -> Result<()> {
    let current_dir = current_dir().with_context(|| "Could not get current directory")?;

    let path = match &upgrade_opts.path {
        Some(path_str) => Path::new(path_str),
        None => &current_dir,
    };

    let mut highlights = collect_highlights(opts, path)?;

    if highlights.is_empty() {
        return Ok(());
    }

    let repository = clippy_repository(opts.quiet)?;

    let rewrites = collect_rewrites(opts, old_channel, new_oid, &repository)?;

    loop {
        let mut rewriters = BTreeMap::new();
        let mut unrewritable_highlights = Vec::new();

        for highlight in &highlights {
            if !rewriters.contains_key(&highlight.file_name) {
                if !backups.contains_key(&highlight.file_name) {
                    let backup = Backup::new(path.join(&highlight.file_name))
                        .with_context(|| format!("Could not backup `{}`", highlight.file_name))?;
                    backups.insert(highlight.file_name.clone(), backup);
                }
                let contents =
                    read_to_string(path.join(&highlight.file_name)).with_context(|| {
                        format!("`read_to_string` failed for `{}`", highlight.file_name)
                    })?;
                // smoelius: Leaking `contents` is a hack.
                let rewriter = Rewriter::new(contents.leak());
                rewriters.insert(highlight.file_name.clone(), (0, rewriter));
            }
            let (last_rewritten_line, rewriter) = rewriters.get_mut(&highlight.file_name).unwrap();
            // smoelius: A `Rewriter`'s rewrites must be in order. So multiple changes to a line
            // must be performed in separate iterations of this loop. A way to circumvent this
            // limitation would be to track columns in a `Highlight`. However, that would likely
            // complicate scoring `Rewrite`s.
            if highlight.line_start <= *last_rewritten_line {
                continue;
            }
            // smoelius: `replacement_source_map` maps replacement text to the rewrites from which
            // they came. A reason for mapping from replacement text is to avoid creating
            // unnecessary distinctions among rewrites. In other words, if two rewrites with the
            // same score provide the same replacement text, then either rewrite can be applied.
            let mut replacement_source_map = applicable_rewrites(&rewrites, highlight)?;
            // smoelius: If the next call to `max` succeeds, it means there is at least one
            // replacement.
            let Some(best_score) = replacement_source_map
                .values()
                .map(|source| source.score)
                .max()
            else {
                // smoelius: An unrewritable, non-primary highlight is not justification for
                // breaking out of the loop.
                if highlight.is_primary {
                    unrewritable_highlights.push((highlight, Reason::None));
                }
                continue;
            };
            replacement_source_map.retain(|_, source| source.score == best_score);
            if replacement_source_map.len() > 1 {
                if highlight.is_primary {
                    unrewritable_highlights.push((
                        highlight,
                        Reason::Multiple(best_score, replacement_source_map),
                    ));
                }
                continue;
            }
            // smoelius: We know there is at least one replacement, because `max` above was not
            // `None`.
            let (replacement, source) = replacement_source_map.pop_first().unwrap();
            // smoelius: Note that a replacement could have multiple sources. But it doesn't really
            // matter, because the replacement text is the same for each. The only way it could
            // matter is that the commit oid would be wrong. But we only track one, so what we
            // output is already inaccurate.
            eprintln!(
                "Rewriting with score {best_score} rewrite from {}: {:#?} {:#?}",
                source.oid.short_id(),
                source.rewrite,
                highlight,
            );
            let _: String = rewriter.rewrite(&source.span, &replacement);
            *last_rewritten_line = source.span.end().line;
        }

        for (file_name, (_, rewriter)) in rewriters {
            let contents = rewriter.contents();
            write(path.join(&file_name), contents)
                .with_context(|| format!("`write` failed for `{file_name}`"))?;
        }

        // smoelius: The existence of unrewritable highlights is not considered an error. For
        // example, there could be an associated non-primary highlight that was rewritten and
        // resolved the warning.
        if !unrewritable_highlights.is_empty() {
            display_unrewritable(&unrewritable_highlights);
            return Ok(());
        }

        highlights = collect_highlights(opts, path)?;

        if highlights.is_empty() {
            return Ok(());
        }
    }
}

fn applicable_rewrites<'rewrite>(
    rewrites: &'rewrite HashMap<Rewrite, Oid>,
    highlight: &Highlight,
) -> Result<ReplacementSourceMap<'rewrite>> {
    let mut replacement_source_map = ReplacementSourceMap::new();
    for (rewrite, &oid) in rewrites {
        if let Some((score, offset)) = rewrite.applicability(highlight) {
            let (_, replacement) = span_and_text_of_tokens(
                1,
                &rewrite.new_lines,
                rewrite.common_prefix_len..rewrite.new_tokens.len() - rewrite.common_suffix_len,
            )?;
            let (span, _) = span_and_text_of_tokens(
                highlight.line_start,
                &highlight.lines,
                offset
                    ..offset
                        + (rewrite.old_tokens.len()
                            - rewrite.common_suffix_len
                            - rewrite.common_prefix_len),
            )?;
            if let Some(source) = replacement_source_map.get_mut(&replacement) {
                // smoelius: Higher scores are better.
                if source.score < score {
                    *source = ReplacementSource {
                        score,
                        span,
                        oid,
                        rewrite,
                    };
                }
            } else {
                replacement_source_map.insert(
                    replacement.clone(),
                    ReplacementSource {
                        score,
                        span,
                        oid,
                        rewrite,
                    },
                );
            }
        }
    }
    Ok(replacement_source_map)
}

/// Returns the span and text of a range of tokens.
///
/// **Warning** If `range` is empty, the returned span is unspecified.
///
/// # Arguments
///
/// - `line_start` is the 1-based line where `lines` start.
/// - `range` is a range of tokens within the tokenization of `lines`.
#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
pub fn span_and_text_of_tokens<S: AsRef<str>>(
    line_start: usize,
    lines: &[S],
    range: Range<usize>,
) -> Result<(Span, String)> {
    // smoelius: If a rewrite strictly removes tokens, `range` will be empty.
    // assert!(!range.is_empty());

    if range.is_empty() {
        return Ok(Default::default());
    }

    let lines = lines.iter().map(AsRef::as_ref).collect::<Vec<_>>();

    let lines_orig = &lines;

    let tokens = tokenize_lines(&lines)?;

    let mut lines = lines.iter();
    let mut line = lines.next().copied().unwrap();

    let mut i_token = 0;
    let mut start = None;
    let mut line_column = LineColumn {
        line: line_start,
        column: 0,
    };
    let mut text = String::new();

    while i_token < range.end {
        if line.as_bytes().iter().all(u8::is_ascii_whitespace) {
            if range.start < i_token {
                text += line;
                text += "\n";
            }
            line = lines.next().unwrap();
            line_column.line += 1;
            line_column.column = 0;
            continue;
        }

        let token = tokens[i_token];
        let len = token.len();

        #[allow(clippy::panic)]
        let offset = line
            .find(token)
            .unwrap_or_else(|| panic!("Could not find token {token:?} in line {line:?}"));

        assert!(
            line.as_bytes()[..offset]
                .iter()
                .all(u8::is_ascii_whitespace)
        );
        if range.start < i_token {
            text += &line[..offset];
        }
        line = &line[offset..];
        line_column.column += offset;

        if range.start == i_token {
            start = Some(line_column);
        }

        assert!(line.starts_with(token));
        if range.start <= i_token {
            text += token;
        }
        line = &line[len..];
        line_column.column += len;

        i_token += 1;
    }

    #[allow(clippy::panic)]
    let start = start.unwrap_or_else(|| {
        panic!(
            "`start` was not set for {:#?}",
            (line_start, lines_orig, line, &text, range),
        )
    });

    Ok((Span::new(start, line_column), text))
}

fn display_unrewritable(unrewritable: &[(&Highlight, Reason)]) {
    for (highlight, reason) in unrewritable {
        assert!(highlight.is_primary);
        match reason {
            Reason::None => {
                eprintln!("Found no applicable rewrites for {highlight:#?}");
            }
            Reason::Multiple(score, rewrites) => {
                eprintln!(
                    "Found multiple rewrites with score {score} for {highlight:#?}: {rewrites:#?}"
                );
            }
        }
    }
}
