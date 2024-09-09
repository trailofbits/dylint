#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_resolve;

use clippy_utils::diagnostics::span_lint;
use pulldown_cmark::{Options, Parser};
use rustc_ast::Attribute;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_resolve::rustdoc::{add_doc_fragment, attrs_to_doc_fragments, DocFragment};
use std::path::{absolute, Path, PathBuf};

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for doc comment links that refer to files outside of their source file's package.
    ///
    /// ### Why is this bad?
    /// Such links will be broken on [docs.rs], for example.
    ///
    /// ### Example
    /// ```rust
    /// //! [general-purpose lints]: ../../general
    /// ```
    /// Use instead:
    /// ```rust
    /// //! [general-purpose lints]: https://github.com/trailofbits/dylint/tree/master/examples/general
    /// ```
    ///
    /// [docs.rs]: https://docs.rs
    pub ESCAPING_DOC_LINK,
    Warn,
    "doc comment links that escape their packages"
}

impl<'tcx> LateLintPass<'tcx> for EscapingDocLink {
    fn check_attribute(&mut self, cx: &LateContext<'tcx>, attr: &'tcx Attribute) {
        let Some(source_path) = cx.sess().local_crate_source_file() else {
            return;
        };

        let Some(manifest_dir) = source_path.local_path().and_then(|path| {
            absolute(path)
                .ok()
                .map(|absolute_path| strip_suffix(&absolute_path, path))
        }) else {
            return;
        };

        assert!(manifest_dir.is_absolute());

        let (fragments, _) = attrs_to_doc_fragments(std::iter::once((attr, None)), true);

        let doc = assemble_doc_fragments(fragments);

        let parser = Parser::new_ext(&doc, Options::all());

        for (_, link_def) in parser.reference_definitions().iter() {
            // smoelius: Heuristic to detect urls (`://`) and intra-doc links (`::`). Is there a
            // better way?
            if link_def.dest.contains("://") || link_def.dest.contains("::") {
                continue;
            }

            let path = link_def
                .dest
                .rsplit_once('#')
                .map_or(link_def.dest.as_ref(), |(prefix, _)| prefix);

            let path = Path::new(path);

            let path = absolutize(&manifest_dir, path, true);

            if path.exists() {
                if !path.starts_with(&manifest_dir) {
                    span_lint(
                        cx,
                        ESCAPING_DOC_LINK,
                        attr.span,
                        "link refers to files outside of the package directory",
                    );
                }
            } else {
                span_lint(cx, ESCAPING_DOC_LINK, attr.span, "broken link");
            }
        }
    }
}

fn strip_suffix(path: &Path, suffix: &Path) -> PathBuf {
    let n = path.components().count();
    let m = suffix.components().count();
    path.components().take(n - m).collect()
}

// smoelius: `assemble_doc_fragments` is based on code from Clippy's `doc-markdown` lint:
// https://github.com/rust-lang/rust-clippy/blob/e88a556124189e3ee23841238252b3831b3af966/clippy_lints/src/doc.rs#L483-L487
fn assemble_doc_fragments(fragments: impl IntoIterator<Item = DocFragment>) -> String {
    let mut doc = String::new();
    for fragment in fragments {
        add_doc_fragment(&mut doc, &fragment);
    }
    doc.pop();
    doc
}

fn absolutize(base: &Path, path: &Path, normalize: bool) -> PathBuf {
    if path.is_absolute() {
        assert!(path.starts_with(base));
        path.to_path_buf()
    } else {
        let path_buf = base.join(path);
        if normalize {
            cargo_util::paths::normalize_path(&path_buf)
        } else {
            path_buf
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
