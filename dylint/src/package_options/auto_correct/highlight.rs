use super::tokenization::{tokenize_fragment, tokenize_lines};
use crate::{error::warn, opts};
use anyhow::Result;
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticSpan};
use dylint_internal::{CommandExt, cargo, rustup::SanitizeEnvironment};
use serde::Deserialize;
use std::{path::Path, time::Instant};

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    #[serde(rename = "message")]
    diagnostic: Option<Diagnostic>,
}

/// Highlighted text from [`DiagnosticSpanLine`]s
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Highlight {
    /// The diagnostic's message
    pub message: String,
    /// The name of the file that the diagnostic comes from
    pub file_name: String,
    /// 1-based line in the file
    pub line_start: usize,
    /// 1-based line in the file
    pub line_end: usize,
    /// Text from the [`DiagnosticSpanLine`]s
    pub lines: Vec<String>,
    /// Tokenized [`DiagnosticSpanLine`]s
    pub tokens: Vec<String>,
    /// Token index where the highlight starts
    pub highlight_start: usize,
    /// Token index where the highlight ends
    pub highlight_end: usize,
    /// Whether the source [`DiagnosticSpan`] was "primary"
    pub is_primary: bool,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Highlight {
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Highlight")
            .field("message", &self.message)
            .field("file_name", &self.file_name)
            .field("line_start", &self.line_start)
            .field("line_end", &self.line_end)
            .field("lines", &self.lines)
            .field("highlight_start", &self.highlight_start)
            .field("highlight_end", &self.highlight_end)
            .field("is_primary", &self.is_primary)
            .finish_non_exhaustive()
    }
}

impl Highlight {
    /// Tries to construct a [`Highlight`] from a [`DiagnosticSpan`]. Fails if the highlighted text
    /// cannot be tokenized.
    fn try_new(message: &str, span: DiagnosticSpan) -> Result<Self> {
        let DiagnosticSpan {
            file_name,
            line_start,
            line_end,
            column_start,
            column_end,
            is_primary,
            text,
            ..
        } = span;

        let lines = text
            .into_iter()
            .map(|span_line| span_line.text)
            .collect::<Vec<_>>();

        let lines_borrowed = lines.iter().map(String::as_str).collect::<Vec<_>>();
        let tokens_borrowed = tokenize_lines(&lines_borrowed)?;
        let tokens = tokens_borrowed
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        assert!(!lines.is_empty());

        // smoelius: The calculations of `highlight_start` and `highlight_end` retokenize parts of
        // the first and last lines. This is kind of ugly.
        let highlight_start = {
            let first_line = &lines_borrowed.first().unwrap()[..column_start - 1];
            let first_line_tokens = tokenize_fragment(first_line)?;
            first_line_tokens.len()
        };
        let highlight_end = {
            let last_line = &lines_borrowed.last().unwrap()[column_end - 1..];
            let last_line_tokens = tokenize_fragment(last_line)?;
            tokens.len() - last_line_tokens.len()
        };

        Ok(Self {
            message: message.to_owned(),
            file_name,
            line_start,
            line_end,
            lines,
            tokens,
            highlight_start,
            highlight_end,
            is_primary,
        })
    }
}

/// Invokes `cargo build` at `path` and returns the generated diagnostic messages as [`Highlight`]s.
pub fn collect_highlights(opts: &opts::Dylint, path: &Path) -> Result<Vec<Highlight>> {
    let start = Instant::now();

    let output = cargo::check("upgraded library package")
        .quiet(opts.quiet)
        .build()
        .sanitize_environment()
        .current_dir(path)
        .arg("--message-format=json")
        .logged_output(false)?;

    let mut highlights = Vec::new();

    if !output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        for result in serde_json::Deserializer::from_str(&stdout).into_iter::<Message>() {
            let message = result?;
            if message.reason != "compiler-message" {
                continue;
            }
            let Some(diagnostic) = message.diagnostic else {
                continue;
            };
            if diagnostic.level == DiagnosticLevel::Error && diagnostic.spans.is_empty() {
                warn(
                    opts,
                    &format!(
                        "Found diagnostic error with no spans: {}",
                        &diagnostic.message
                    ),
                );
                continue;
            }
            for span in diagnostic.spans {
                if span.text.is_empty() {
                    continue;
                }
                let highlight = Highlight::try_new(&diagnostic.message, span)?;
                assert!(!highlight.tokens.is_empty());
                highlights.push(highlight);
            }
        }

        highlights.sort();
    }

    let elapsed = start.elapsed();
    eprintln!(
        "Found {} highlights in {} seconds",
        highlights.len(),
        elapsed.as_secs()
    );

    Ok(highlights)
}
