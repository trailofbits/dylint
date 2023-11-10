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
use cargo_metadata::{Metadata, MetadataCommand};
use clippy_utils::{diagnostics::span_lint_and_help, source::snippet_opt};
use dylint_internal::env::var;
use if_chain::if_chain;
use once_cell::sync::OnceCell;
use rustc_ast::ast::{Attribute, MetaItem, NestedMetaItem};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{
    Block, Expr, ExprKind, HirId, ImplItem, Item, ItemKind, Node, Stmt, StmtKind, CRATE_HIR_ID,
};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, impl_lint_pass, Session};
use rustc_span::{sym, BytePos, CharPos, FileLines, FileName, RealFileName, Span, Symbol};
use rustfix::diagnostics::{Diagnostic, DiagnosticSpan};
use serde::Deserialize;
use std::{
    cell::RefCell,
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
    /// - Recommends to reduce to the following scopes only (not arbitrary inner scopes):
    ///   - item
    ///   - trait item
    ///   - `impl` item
    ///   - statement
    ///   - expression at the end of a block
    /// - Cannot see inside `#[test]` functions, i.e., does not recommend to reduce to a scope
    ///   smaller than an entire test.
    /// - `--force-warn` does not override `clippy.toml` settings. So if `allow-unwrap-in-tests` is
    ///   set to `true`, `overscoped_allow` will not recommend to reduce scopes inside modules
    ///   marked with `#[cfg(test)]`, for example.
    ///
    /// ### How to use this lint
    /// Two steps are required:
    /// 1. For the lint whose `allow` scopes you want to check, run it at the [`force-warn`] level
    ///    and store the resulting warnings in a file called `warnings.json`. For example, to check
    ///    the scopes of `allow(clippy::unwrap_used)`, you might run the following command:
    ///
    ///    ```sh
    ///    cargo clippy --message-format=json -- --force-warn clippy::unwrap-used > warnings.json
    ///    ```
    ///
    ///    To perform a similar check for the Dylint lint `non_thread_safe_call_in_test`, you might
    ///    run the following command:
    ///
    ///    ```sh
    ///    DYLINT_RUSTFLAGS='--force-warn non_thread_safe_call_in_test' cargo dylint \
    ///       --lib non_thread_safe_call_in_test -- --message-format=json > warnings.json
    ///    ```
    ///
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
    metadata: OnceCell<Metadata>,
    diagnostics: Vec<Diagnostic>,
    ancestor_meta_item_span_map: FxHashMap<HirId, FxHashMap<Span, FxHashSet<Option<Span>>>>,
    canonical_paths_cache: RefCell<FxHashMap<PathBuf, PathBuf>>,
}

impl_lint_pass!(OverscopedAllow => [OVERSCOPED_ALLOW]);

#[derive(Debug, Deserialize)]
struct Message {
    reason: String,
    message: Option<Diagnostic>,
}

#[allow(clippy::no_mangle_with_rust_abi)]
#[no_mangle]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    let diagnostics = match read_diagnostics() {
        Ok(diagnostics) => diagnostics,
        Err(error) => {
            sess.warn(format!("`overscoped_allow` is disabled: {error:?}"));
            Vec::new()
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
        if message.reason == "compiler-message"
            && let Some(diagnostic) = message.message
        {
            diagnostics.push(diagnostic);
        }
    }
    Ok(diagnostics)
}

impl OverscopedAllow {
    fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            metadata: OnceCell::new(),
            diagnostics,
            ancestor_meta_item_span_map: FxHashMap::default(),
            canonical_paths_cache: RefCell::new(FxHashMap::default()),
        }
    }

    fn metadata(&self, source_path_sample: &Path) -> &Metadata {
        self.metadata.get_or_init(|| {
            let parent = source_path_sample.parent().unwrap();
            let source_dir = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            MetadataCommand::new()
                .current_dir(source_dir)
                .no_deps()
                .exec()
                .unwrap()
        })
    }
}

impl<'tcx> LateLintPass<'tcx> for OverscopedAllow {
    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.visit(cx, item.hir_id());
    }

    fn check_impl_item_post(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        self.visit(cx, impl_item.hir_id());
    }

    // smoelius: `LateLintPass` does not currently have `check_stmt_post` or `check_trait_item_post`
    // methods:
    // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/passes/trait.LateLintPass.html
    fn check_block_post(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        for stmt in block.stmts {
            self.visit(cx, stmt.hir_id);
        }

        if let Some(expr) = block.expr {
            self.visit(cx, expr.hir_id);
        }

        // smoelius: The grandparent is the potential trait item (in which case, the parent is an
        // `Expr`).
        if let Node::TraitItem(trait_item) = cx
            .tcx
            .hir()
            .get_parent(cx.tcx.hir().parent_id(block.hir_id))
        {
            self.visit(cx, trait_item.hir_id());
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        self.emit(cx, CRATE_HIR_ID);
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
            if self.span_contains_diagnostic(cx, span, &self.diagnostics[i]) {
                let diagnostic = self.diagnostics.swap_remove(i);
                self.check_ancestor_lint_attrs(cx, hir_id, &diagnostic);
            } else {
                i += 1;
            }
        }
    }

    fn check_ancestor_lint_attrs(
        &mut self,
        cx: &LateContext<'_>,
        hir_id: HirId,
        diagnostic: &Diagnostic,
    ) {
        let started_in_test = is_extern_crate_test(cx, hir_id);
        let mut target_hir_id = None;

        for ancestor_hir_id in std::iter::once(hir_id)
            .chain(cx.tcx.hir().parent_iter(hir_id).map(|(hir_id, _)| hir_id))
        {
            if !can_have_attrs(cx, ancestor_hir_id) {
                continue;
            }

            if target_hir_id.is_none() {
                target_hir_id = Some(ancestor_hir_id);
            }

            for attr in cx.tcx.hir().attrs(ancestor_hir_id) {
                if !is_lint_attr(attr) {
                    continue;
                }
                if let Some(meta_item) = meta_item_for_diagnostic(attr, diagnostic) {
                    if attr.has_name(sym::allow) {
                        let target_span = target_hir_id.and_then(|target_hir_id| {
                            if target_hir_id == ancestor_hir_id {
                                None
                            } else {
                                Some(cx.tcx.hir().span(target_hir_id))
                            }
                        });
                        let meta_item_span_map = self
                            .ancestor_meta_item_span_map
                            .entry(ancestor_hir_id)
                            .or_default();
                        let spans = meta_item_span_map.entry(meta_item.span).or_default();
                        spans.insert(target_span);
                    } else {
                        // smoelius: Don't alert if we started in a test. The `allow` could have
                        // appeared inside the test, and `overscoped_allow` currently cannot see
                        // inside tests.
                        assert!(
                            started_in_test || attr.has_name(sym::expect),
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
                // smoelius: If a span is `None`, it means we could not find a `Node` satisfying
                // `can_have_attrs` between the diagnostic source (inclusive) and the `allow`
                // (exclusive). This is likely due to `can_have_attrs` being incomplete.
                if let [Some(span)] = spans.iter().collect::<Vec<_>>().as_slice() {
                    let span = span.with_hi(span.lo());
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

    fn span_contains_diagnostic(
        &self,
        cx: &LateContext<'_>,
        span: Span,
        diagnostic: &Diagnostic,
    ) -> bool {
        diagnostic
            .spans
            .iter()
            .all(|diagnostic_span| self.span_contains_diagnostic_span(cx, span, diagnostic_span))
    }

    fn span_contains_diagnostic_span(
        &self,
        cx: &LateContext<'_>,
        span: Span,
        diagnostic_span: &DiagnosticSpan,
    ) -> bool {
        let Some(span_local_path) = local_path_from_span(cx, span) else {
            return false;
        };
        let metadata = self.metadata(&span_local_path);
        let lhs = &span_local_path;
        let rhs = Path::new(&diagnostic_span.file_name);
        for path in [lhs, rhs] {
            if_chain! {
                if self.canonical_paths_cache.borrow().get(path).is_none();
                if let Ok(canonical_path) = if path.is_absolute() {
                    path.canonicalize()
                } else {
                    metadata.workspace_root.as_std_path().join(path).canonicalize()
                };
                then {
                    self.canonical_paths_cache
                        .borrow_mut()
                        .insert(path.to_path_buf(), canonical_path);
                }
            }
        }
        if_chain! {
            if let Some(lhs) = self.canonical_paths_cache.borrow().get(lhs);
            if let Some(rhs) = self.canonical_paths_cache.borrow().get(rhs);
            if lhs == rhs;
            if let Ok(FileLines { lines, .. }) = cx.sess().source_map().span_to_lines(span);
            if let Some(first_line) = lines.first();
            if let Some(last_line) = lines.last();
            then {
                (first_line.line_index + 1 < diagnostic_span.line_start
                    || (first_line.line_index + 1 == diagnostic_span.line_start
                        && first_line.start_col + CharPos(1)
                            <= CharPos(diagnostic_span.column_start)))
                    && (diagnostic_span.line_end < last_line.line_index + 1
                        || (diagnostic_span.line_end == last_line.line_index + 1
                            && CharPos(diagnostic_span.column_end)
                                <= last_line.end_col + CharPos(1)))
            } else {
                false
            }
        }
    }
}

fn include_trailing_semicolons(cx: &LateContext<'_>, mut span: Span) -> Span {
    // smoelius: I have seen `span_to_lines` fail on real code.
    let Ok(FileLines { file, .. }) = cx.sess().source_map().span_to_lines(span) else {
        return span;
    };
    while span.hi() < file.end_position() {
        let next = span.with_hi(span.hi() + BytePos(1));
        if !snippet_opt(cx, next).map_or(false, |snip| snip.ends_with(';')) {
            break;
        }
        span = next;
    }
    span
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

fn is_extern_crate_test(cx: &LateContext<'_>, hir_id: HirId) -> bool {
    let node = cx.tcx.hir().get(hir_id);
    if let Node::Item(Item {
        kind: ItemKind::ExternCrate(None),
        ident,
        ..
    }) = node
    {
        ident.as_str() == "test"
    } else {
        false
    }
}

// smoelius: `can_have_attrs` is not complete.
fn can_have_attrs(cx: &LateContext<'_>, hir_id: HirId) -> bool {
    let node = cx.tcx.hir().get(hir_id);

    if matches!(node, Node::Item(_) | Node::TraitItem(_) | Node::ImplItem(_)) {
        return true;
    }

    if let Node::Stmt(Stmt {
        kind: StmtKind::Semi(Expr {
            kind: expr_kind, ..
        }),
        ..
    })
    | Node::Expr(Expr {
        kind: expr_kind, ..
    }) = node
    {
        // smoelius: Attributes have the same precedence as unary operators:
        // https://github.com/rust-lang/rust/issues/15701#issuecomment-138092406
        // Expression precedence is documented here:
        // https://doc.rust-lang.org/reference/expressions.html#expression-precedence
        return matches!(
            expr_kind,
            ExprKind::Path(_)
                | ExprKind::MethodCall(..)
                | ExprKind::Field(..)
                | ExprKind::Call(..)
                | ExprKind::Index(..)
                | ExprKind::Unary(..),
        );
    }

    // smoelius: Accept all non-semi statements.
    if matches!(node, Node::Stmt(_)) {
        return true;
    }

    false
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
    use std::{env::set_var, process::Command, sync::Mutex};
    use tempfile::NamedTempFile;

    static MUTEX: Mutex<()> = Mutex::new(());

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn ui_general() {
        let _lock = MUTEX.lock().unwrap();

        install_clippy();

        let (file, temp_path) = NamedTempFile::new().unwrap().into_parts();
        Command::new("cargo")
            .args([
                "clippy",
                "--example=ui_general",
                "--message-format=json",
                "--",
                "--force-warn=clippy::module-name-repetitions",
                "--force-warn=clippy::unused-self",
                "--force-warn=clippy::unwrap-used",
                "--force-warn=clippy::wrong-self-convention",
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
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_general"),
        );
    }

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn ui_test() {
        let _lock = MUTEX.lock().unwrap();

        install_clippy();

        let (file, temp_path) = NamedTempFile::new().unwrap().into_parts();
        Command::new("cargo")
            .args([
                "clippy",
                "--tests",
                "--message-format=json",
                "--",
                "--force-warn=clippy::panic",
            ])
            .stdout(file)
            .assert()
            .success();
        set_var(
            OVERSCOPED_ALLOW_PATH,
            temp_path.to_string_lossy().to_string(),
        );
        dylint_testing::ui::Test::src_base(
            env!("CARGO_PKG_NAME"),
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_test"),
        )
        .rustc_flags(["--test"])
        .run();
    }

    // smoelius: I am not sure why, but I started seeing `error: 'cargo-clippy' is not installed for
    // the toolchain...` after consolidating all of the restriction lints under one workspace.
    fn install_clippy() {
        Command::new("rustup")
            .args(["component", "add", "clippy"])
            .assert()
            .success();
    }
}
