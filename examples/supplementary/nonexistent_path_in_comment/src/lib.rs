#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use cargo_metadata::MetadataCommand;
use clippy_utils::diagnostics::span_lint_and_help;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::{BytePos, FileName, Span, SyntaxContext};

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

static LINE_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(^|[^/])(//[/!]?.*)").unwrap());
static BLOCK_COMMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"/\*((?:[^*]|\*[^/])*)\*/").unwrap());
static PATH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"((?:\./|\.\./|/|[\w/\$-]+/)+[\w$-]+(?:\.[\w-]+)+)").unwrap());
static URL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+").unwrap());

impl<'tcx> LateLintPass<'tcx> for NonexistentPathInComment {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.tcx.sess.source_map();

        for file in source_map.files().iter() {
            if let Some(content) = file.src.as_ref() {
                let file_start = file.start_pos;

                // Process line comments
                for cap in LINE_COMMENT.captures_iter(content) {
                    if let Some(comment_text) = cap.expect("expected capture").get(2) {
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
                    if let Some(comment_text) = cap.expect("expected capture").get(1) {
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

    let mut url_ranges = Vec::new();
    for mat in URL_REGEX.find_iter(comment_text) {
        if let Ok(m) = mat {
            url_ranges.push(m.start()..m.end());
        }
    }

    let mut non_url_ranges = Vec::new();
    let mut last_end = 0;
    for range in url_ranges {
        if last_end < range.start {
            non_url_ranges.push(last_end..range.start);
        }
        last_end = range.end;
    }
    if last_end < comment_text.len() {
        non_url_ranges.push(last_end..comment_text.len());
    }

    for caps in PATH_REGEX.captures_iter(comment_text) {
        let path_match = caps.expect("no capture").get(1).unwrap();
        let match_start = path_match.start();
        let match_end = path_match.end();
        let path_str = path_match.as_str();

        if non_url_ranges
            .iter()
            .any(|r| r.start <= match_start && match_end <= r.end)
        {
            if path_str.starts_with("http://")
                || path_str.starts_with("https://")
                || path_str.starts_with("www.")
                || path_str.starts_with('$')
            {
                continue;
            }

            let full_path = base_dir.join(path_str);
            if full_path.exists() {
                continue;
            }

            if let Some(root_pkg) = metadata.root_package() {
                if let Some(manifest_parent) = root_pkg.manifest_path.parent() {
                    let manifest_dir = manifest_parent.as_std_path();

                    let candidate_from_root =
                        if let Some(stripped) = path_str.strip_prefix(&root_pkg.name) {
                            let stripped = stripped.strip_prefix('/').unwrap_or(stripped);
                            manifest_dir.join(stripped)
                        } else {
                            manifest_dir.join(path_str)
                        };

                    if candidate_from_root.exists() {
                        continue;
                    }
                }
            }

            let path_span = Span::new(
                span.lo() + BytePos(match_start as u32),
                span.lo() + BytePos(match_end as u32),
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
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
