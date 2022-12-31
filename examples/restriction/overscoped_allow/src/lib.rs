#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use anyhow::{Context, Result};
use clippy_utils::{diagnostics::span_lint_and_help, source::snippet_opt};
use dylint_internal::env::var;
use if_chain::if_chain;
use rustc_ast::ast::{Attribute, MetaItem, NestedMetaItem};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{Block, HirId, ImplItem, Item};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, impl_lint_pass, Session};
use rustc_span::{sym, BytePos, CharPos, FileLines, FileName, RealFileName, Span, Symbol};
use rustfix::diagnostics::{Diagnostic, DiagnosticSpan};
use serde::Deserialize;
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

const OVERSCOPED_ALLOW_PATH: &str = "OVERSCOPED_ALLOW_PATH";

declare_lint! {
    /// ### What it does
    /// Checks for `allow` attributes whose scope could be reduced.
    ///
    /// ### Why is this bad?
    /// An `allow` attribute whose scope is too large could suppress warnings/errors and cause them
    /// to go unnoticed.
    ///
    /// ### Known problems
    /// Recommends to reduce to item, `Impl` item, and statement scopes only (not arbitrary inner
    /// scopes).
    ///
    /// ### How to use this lint
    /// Two steps are required:
    /// 1. For the lint whose `allow` scopes you want to check, run it at the [`force-warn`] level
    ///    and store the resulting warnings in a file called `warnings.json`. For example, to check
    ///    the scopes of `allow(clippy::unwrap_used)`, you might run the following command:
    ///    ```sh
    ///    cargo clippy --message-format=json -- --force-warn clippy::unwrap-used > warnings.json
    ///    ```
    ///    To perform a similar check for the Dylint lint `non_thread_safe_call_in_test`, you might
    ///    run the following command:
    ///    ```sh
    ///    DYLINT_RUSTFLAGS='--force-warn non_thread_safe_call_in_test' cargo dylint \
    ///       --lib non_thread_safe_call_in_test -- --message-format=json > warnings.json
    ///    ```
    /// 2. Run the `overscoped_allow` lint. The lint will find and use the `warnings.json` file
    ///    generated in 1.
    ///
    /// To use a file other than `warnings.json`, store that file's path in the environment variable
    /// variable `OVERSCOPED_ALLOW_PATH`.
    ///
    /// ### Example
    /// ```rust
    /// #[allow(clippy::module_name_repetitions)]
    /// mod cake {
    ///     struct BlackForestCake;
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// mod cake {
    ///     #[allow(clippy::module_name_repetitions)]
    ///     struct BlackForestCake;
    /// }
    /// ```
    ///
    /// [`force-warn`]: https://doc.rust-lang.org/rustc/lints/levels.html#force-warn
    pub OVERSCOPED_ALLOW,
    Warn,
    "`allow` attributes whose scope could be reduced"
}

#[derive(Default)]
struct OverscopedAllow {
    diagnostics: Vec<Diagnostic>,
    ancestor_meta_item_span_map: FxHashMap<HirId, FxHashMap<Span, FxHashSet<Span>>>,
}

impl_lint_pass!(OverscopedAllow => [OVERSCOPED_ALLOW]);

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    message: Option<Diagnostic>,
}

#[no_mangle]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    let diagnostics = match read_diagnostics() {
        Ok(diagnostics) => diagnostics,
        Err(error) => {
            sess.warn(format!("`overscoped_allow` is disabled: {error:?}"));
            return;
        }
    };

    lint_store.register_lints(&[OVERSCOPED_ALLOW]);
    lint_store.register_late_pass(move |_| Box::new(OverscopedAllow::new(diagnostics.clone())));
}

fn read_diagnostics() -> Result<Vec<Diagnostic>> {
    let path = var(OVERSCOPED_ALLOW_PATH)
        .ok()
        .unwrap_or_else(|| "warnings.json".to_owned());
    let canonical_path = Path::new(&path)
        .canonicalize()
        .with_context(|| format!("Could not canonicalize {path:?}"))?;
    let file = OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Could not open {canonical_path:?}"))?;
    let mut diagnostics = Vec::new();
    for result in serde_json::Deserializer::from_reader(file).into_iter::<Message>() {
        let message = result?;
        if message.reason == "compiler-message" && let Some(diagnostic) = message.message {
            diagnostics.push(diagnostic);
        }
    }
    Ok(diagnostics)
}

impl OverscopedAllow {
    fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            ancestor_meta_item_span_map: FxHashMap::default(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for OverscopedAllow {
    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.visit(cx, item.hir_id());
    }

    fn check_impl_item_post(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        self.visit(cx, impl_item.hir_id());
    }

    fn check_block_post(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        for stmt in block.stmts {
            self.visit(cx, stmt.hir_id);
        }
    }
}

impl OverscopedAllow {
    fn visit(&mut self, cx: &LateContext<'_>, hir_id: HirId) {
        self.check(cx, hir_id);
        self.emit(cx, hir_id);
    }

    fn check(&mut self, cx: &LateContext<'_>, hir_id: HirId) {
        let span = include_trailing_semicolons(cx, cx.tcx.hir().span(hir_id));
        let mut i = 0;
        while i < self.diagnostics.len() {
            if span_contains_diagnostic(cx, span, &self.diagnostics[i]) {
                let diagnostic = self.diagnostics.swap_remove(i);
                self.check_ancestor_lint_attrs(cx, hir_id, span, &diagnostic);
            } else {
                i += 1;
            }
        }
    }

    fn check_ancestor_lint_attrs(
        &mut self,
        cx: &LateContext<'_>,
        hir_id: HirId,
        span: Span,
        diagnostic: &Diagnostic,
    ) {
        for ancestor_hir_id in std::iter::once(hir_id)
            .chain(cx.tcx.hir().parent_iter(hir_id).map(|(hir_id, _)| hir_id))
        {
            for attr in cx.tcx.hir().attrs(ancestor_hir_id) {
                if !is_lint_attr(attr) {
                    continue;
                }
                if let Some(meta_item) = meta_item_for_diagnostic(attr, diagnostic) {
                    if attr.has_name(sym::allow) {
                        if hir_id != ancestor_hir_id {
                            let meta_item_span_map = self
                                .ancestor_meta_item_span_map
                                .entry(ancestor_hir_id)
                                .or_default();
                            let spans = meta_item_span_map.entry(meta_item.span).or_default();
                            spans.insert(span.with_hi(span.lo()));
                        }
                    } else {
                        assert!(
                            attr.has_name(sym::expect),
                            "Could not find `allow` for diagnostic: {diagnostic:?}"
                        );
                    }
                    return;
                }
            }
        }
    }

    fn emit(&mut self, cx: &LateContext<'_>, hir_id: HirId) {
        if let Some(meta_item_span_map) = self.ancestor_meta_item_span_map.remove(&hir_id) {
            for (meta_item_span, spans) in meta_item_span_map {
                // smoelius: Don't warn about `allow`s spanning multiple diagnostics.
                if spans.len() == 1 {
                    for span in spans {
                        span_lint_and_help(
                            cx,
                            OVERSCOPED_ALLOW,
                            meta_item_span,
                            "`allow` could be moved closer to diagnostic source",
                            Some(span),
                            "`allow` could be moved here",
                        );
                    }
                }
            }
        }
    }
}

fn include_trailing_semicolons(cx: &LateContext<'_>, mut span: Span) -> Span {
    let FileLines { file, .. } = cx.sess().source_map().span_to_lines(span).unwrap();
    while span.hi() < file.end_pos {
        let next = span.with_hi(span.hi() + BytePos(1));
        if !snippet_opt(cx, next).map_or(false, |snip| snip.ends_with(';')) {
            break;
        }
        span = next;
    }
    span
}

fn span_contains_diagnostic(cx: &LateContext<'_>, span: Span, diagnostic: &Diagnostic) -> bool {
    diagnostic
        .spans
        .iter()
        .all(|diagnostic_span| span_contains_diagnostic_span(cx, span, diagnostic_span))
}

fn span_contains_diagnostic_span(
    cx: &LateContext<'_>,
    span: Span,
    diagnostic_span: &DiagnosticSpan,
) -> bool {
    if_chain! {
        if let Some(lhs) = local_path_from_span(cx, span).and_then(|path| path.canonicalize().ok());
        if let Ok(rhs) = Path::new(&diagnostic_span.file_name).canonicalize();
        if lhs == rhs;
        let FileLines { lines, .. } = cx.sess().source_map().span_to_lines(span).unwrap();
        if let Some(first_line) = lines.first();
        if let Some(last_line) = lines.last();
        then {
            (first_line.line_index + 1 < diagnostic_span.line_start
                || (first_line.line_index + 1 == diagnostic_span.line_start
                    && first_line.start_col + CharPos(1) <= CharPos(diagnostic_span.column_start)))
                && (diagnostic_span.line_end < last_line.line_index + 1
                    || (diagnostic_span.line_end == last_line.line_index + 1
                        && CharPos(diagnostic_span.column_end) <= last_line.end_col + CharPos(1)))
        } else {
            false
        }
    }
}

fn local_path_from_span(cx: &LateContext<'_>, span: Span) -> Option<PathBuf> {
    if let FileName::Real(RealFileName::LocalPath(local_path)) =
        cx.sess().source_map().span_to_filename(span)
    {
        Some(local_path)
    } else {
        None
    }
}

fn is_lint_attr(attr: &Attribute) -> bool {
    attr.ident()
        .map_or(false, |ident| is_lint_level(ident.name))
}

// smoelius: `is_lint_level` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/11434f270fca0e403b695e99d4bf0a7212c46f14/clippy_lints/src/attrs.rs#L759-L761
#[allow(clippy::missing_const_for_fn)]
fn is_lint_level(symbol: Symbol) -> bool {
    matches!(
        symbol,
        sym::allow | sym::expect | sym::warn | sym::deny | sym::forbid
    )
}

fn meta_item_for_diagnostic(attr: &Attribute, diagnostic: &Diagnostic) -> Option<MetaItem> {
    if_chain! {
        if let Some(items) = attr.meta_item_list();
        if let Some(code) = &diagnostic.code;
        then {
            items
                .iter()
                .filter_map(NestedMetaItem::meta_item)
                .find(|meta_item| {
                    meta_item
                        .path
                        .segments
                        .iter()
                        .map(|path_segment| path_segment.ident.as_str())
                        .collect::<Vec<_>>()
                        .join("::")
                        == code.code
                })
                .cloned()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::OVERSCOPED_ALLOW_PATH;
    use assert_cmd::prelude::*;
    use std::{env::set_var, process::Command};
    use tempfile::NamedTempFile;

    #[test]
    fn ui() {
        let (file, temp_path) = NamedTempFile::new().unwrap().into_parts();
        Command::new("cargo")
            .args([
                "clippy",
                "--examples",
                "--message-format=json",
                "--",
                "--force-warn=clippy::module-name-repetitions",
                "--force-warn=clippy::unused-self",
                "--force-warn=clippy::unwrap-used",
            ])
            .stdout(file)
            .assert()
            .success();
        set_var(
            OVERSCOPED_ALLOW_PATH,
            temp_path.to_string_lossy().to_string(),
        );
        // smoelius: Don't use `dylint_testing::ui_test_example`. That function copies the example's
        // source file to a temporary directory, so the resulting path wouldn't match what's in the
        // (temporary) `warnings.json` file.
        dylint_testing::ui_test(
            env!("CARGO_PKG_NAME"),
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
        );
    }
}
