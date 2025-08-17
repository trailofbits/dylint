#![feature(rustc_private)]
#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(nonexistent_path_in_comment))]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use cargo_metadata::MetadataCommand;
use clippy_utils::diagnostics::span_lint_and_help;
use regex::{Match, Regex};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::{BytePos, FileName, Span, SyntaxContext};
use std::sync::LazyLock;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// This lint checks code comments, including both line comments (using `//`) and block comments
    /// (`/*...*/`) for file path references. It then validates that the referenced files exist either
    /// relative to the source file's directory or relative to the workspace root. When a file path
    /// reference does not point to an existing file, the lint emits a warning.
    ///
    /// ### Why is this bad?
    ///
    /// References to nonexistent files in comments can be misleading:
    ///
    /// - They clutter the code with outdated or inaccurate references.
    /// - They may cause confusion among developers who are trying to trace implementation details
    ///   or documentation.
    ///
    /// ### Known problems
    ///
    /// Currently, this lint must be allowed at the crate level.
    ///
    /// - This example:
    ///
    /// ```rust
    /// // dylint/dylint/build.rs  (it exists)
    /// ```
    ///
    /// would get flagged here because the workspace root is `supplementary`
    /// it did exist, as this lint doesn't check for project root.
    ///
    /// ### Example
    ///
    /// ```
    /// // See ../nonexistent/path/file.rs for implementation details
    /// fn main() {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```
    /// // See ../actual/path/file.rs for implementation details
    /// fn main() {}
    /// ```
    pub NONEXISTENT_PATH_IN_COMMENT,
    Warn,
    "file paths in comments that do not exist"
}

// smoelius: Require at least two '/' to consider a string a path.
const MIN_PATH_SEPARATORS: usize = 2;

static LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(^|[^/])(//([^/].*))").unwrap());
static BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/\*(([^*]|\*[^/])*)\*/").unwrap());
static PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-./\w:]+").unwrap());

impl<'tcx> LateLintPass<'tcx> for NonexistentPathInComment {
    #[allow(clippy::cast_possible_truncation)]
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.tcx.sess.source_map();

        for file in source_map.files().iter() {
            if let Some(content) = file.src.as_ref() {
                let file_start = file.start_pos;

                for cap in LINE_COMMENT.captures_iter(content) {
                    // smoelius: If the "//" is preceded by ':', assume it is part of a url (e.g.,
                    // "https://").
                    if cap.get(1).as_ref().map(Match::as_str) == Some(":") {
                        continue;
                    }
                    if let Some(comment_text) = cap.get(3) {
                        check_comment(
                            cx,
                            Span::new(
                                file_start + BytePos(comment_text.start() as u32),
                                file_start + BytePos(comment_text.end() as u32),
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
                                file_start + BytePos(comment_text.start() as u32),
                                file_start + BytePos(comment_text.end() as u32),
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

#[allow(clippy::cast_possible_truncation)]
fn check_comment(cx: &LateContext<'_>, span: Span, comment_text: &str, filename: &FileName) {
    let base_dir = match filename {
        FileName::Real(real_filename) => real_filename
            .local_path()
            .expect("failed getting path")
            .parent()
            .unwrap()
            .to_path_buf(),
        _ => return,
    };

    let Ok(metadata) = MetadataCommand::new()
        .current_dir(&base_dir)
        .no_deps()
        .exec()
    else {
        return;
    };

    for caps in PATH_REGEX.captures_iter(comment_text) {
        let mut path_str = &caps[0];

        if path_str.chars().filter(|&c| c == '/').count() < MIN_PATH_SEPARATORS {
            continue;
        }

        if path_str.starts_with("http://") || path_str.starts_with("https://") {
            continue;
        }

        // smoelius: Strip line and column references.
        let last_slash = path_str.rfind('/').unwrap();
        if let Some(index) = path_str[last_slash..].find(':') {
            path_str = &path_str[..last_slash + index];
        }

        let full_path = base_dir.join(path_str);

        if full_path.exists() {
            continue;
        }

        let candidate_from_root = metadata.workspace_root.join(path_str);

        if candidate_from_root.exists() {
            continue;
        }

        let path_start = caps.get(0).unwrap().start();
        let path_end = path_start + path_str.len();
        let path_span = Span::new(
            span.lo() + BytePos(path_start as u32),
            span.lo() + BytePos(path_end as u32),
            span.ctxt(),
            None,
        );
        span_lint_and_help(
            cx,
            NONEXISTENT_PATH_IN_COMMENT,
            path_span,
            "referenced path does not exist",
            None,
            "verify the path is correct or remove the reference",
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
