#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use cargo_metadata::MetadataCommand;
use clippy_utils::diagnostics::span_lint_and_help;
use regex::Regex;
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

static LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(^|[^/])(//([^/].*))").unwrap());
static BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/\*(([^*]|\*[^/])*)\*/").unwrap());
static PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"((?:\./|\.\./|/|[\w/-]+/)+[\w-]+(?:\.[\w-]+)+)").unwrap());

impl<'tcx> LateLintPass<'tcx> for NonexistentPathInComment {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.tcx.sess.source_map();

        for file in source_map.files().iter() {
            if let Some(content) = file.src.as_ref() {
                let file_start = file.start_pos;

                for cap in LINE_COMMENT.captures_iter(content) {
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

    let metadata = MetadataCommand::new()
        .current_dir(&base_dir)
        .no_deps()
        .exec()
        .expect("failed getting metadata");

    for caps in PATH_REGEX.captures_iter(comment_text) {
        let path_str = &caps[1];
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

        let path_start = caps.get(1).unwrap().start();
        let path_end = caps.get(1).unwrap().end();
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
