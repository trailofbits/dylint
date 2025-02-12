#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use once_cell::sync::Lazy;
use regex::Regex;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::{Span, SyntaxContext};
use std::path::Path;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for file paths in comments that don't exist in the filesystem.
    ///
    /// ### Why is this bad?
    /// References to nonexistent files in comments can be misleading and may indicate outdated
    /// documentation or typos.
    ///
    /// ### Example
    /// ```
    /// // See ../nonexistent/path/file.rs for implementation details
    /// fn main() {}
    /// ```
    /// Use instead:
    /// ```
    /// // See ../actual/path/file.rs for implementation details
    /// fn main() {}
    /// ```
    pub NONEXISTENT_PATH_IN_COMMENT,
    Warn,
    "reports file paths in comments that do not exist"
}

static LINE_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new("(^|[^/])(//([^/].*))").unwrap());
static BLOCK_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"/\*(([^*]|\*[^/])*)\*/").unwrap());
static PATH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"((?:\.\./|/|[\w/-]+/)+[\w-]+(?:\.[\w-]+)+)").unwrap());

impl<'tcx> LateLintPass<'tcx> for NonexistentPathInComment {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.tcx.sess.source_map();

        for file in source_map.files().iter() {
            if let Ok(content) = file.src.as_ref().ok_or("err") {
                let file_start = file.start_pos;

                for cap in LINE_COMMENT.captures_iter(content) {
                    if let Some(comment_text) = cap.get(3) {
                        check_comment(
                            cx,
                            Span::new(
                                file_start + rustc_span::BytePos(comment_text.start() as u32),
                                file_start + rustc_span::BytePos(comment_text.end() as u32),
                                SyntaxContext::root(),
                                None,
                            ),
                            comment_text.as_str(),
                            &file.name,
                        );
                    }
                }

                for cap in BLOCK_COMMENT.captures_iter(content) {
                    if let Some(comment_text) = cap.get(1) {
                        check_comment(
                            cx,
                            Span::new(
                                file_start + rustc_span::BytePos(comment_text.start() as u32),
                                file_start + rustc_span::BytePos(comment_text.end() as u32),
                                SyntaxContext::root(),
                                None,
                            ),
                            comment_text.as_str(),
                            &file.name,
                        );
                    }
                }
            }
        }
    }
}

fn check_comment(
    cx: &LateContext<'_>,
    span: Span,
    comment_text: &str,
    filename: &rustc_span::FileName,
) {
    let base_dir = match filename {
        rustc_span::FileName::Real(real_filename) => Path::new(real_filename.local_path().unwrap())
            .parent()
            .unwrap()
            .to_path_buf(),
        _ => return,
    };

    for captures in PATH_REGEX.captures_iter(comment_text) {
        let path_str = &captures[1];
        let full_path = base_dir.join(path_str);

        if !full_path.exists() {
            span_lint_and_help(
                cx,
                NONEXISTENT_PATH_IN_COMMENT,
                span,
                format!("referenced path does not exist: {}", path_str),
                None,
                "verify the path is correct or remove the reference",
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
