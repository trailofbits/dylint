#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;

use clippy_utils::diagnostics::span_lint_and_then;
use dylint_internal::match_def_path;
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::CoroutineLayout;
use rustc_middle::ty::Adt;

dylint_linting::declare_late_lint! {
    /// This lint is due to David Barsky (@davidbarsky).
    ///
    /// ### What it does
    ///
    /// Checks for calls to await while holding a
    /// `tracing` span's `Entered` or `EnteredSpan` guards.
    ///
    /// ### Why is this bad?
    ///
    /// The guards created by `tracing::Span::enter()` or `tracing::Span::entered()` across
    /// `.await` points will result in incorrect traces. This occurs when an async function or
    /// async block yields at an .await point, the current scope is exited, but values in that scope
    /// are not dropped (because the async block will eventually resume execution from that
    /// await point). This means that another task will begin executing while remaining in the
    /// entered span.
    ///
    /// ### Known problems
    ///
    /// Will report false positive for explicitly dropped refs ([#6353]).
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// use tracing::{span, Level};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     THIS WILL RESULT IN INCORRECT TRACES
    ///     let _enter = span.enter();
    ///     bar().await;
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// use tracing::{span, Level}
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     let some_value = span.in_scope(|| {
    ///         // run some synchronous code inside the span...
    ///     });
    ///
    ///     // This is okay! The span has already been exited before we reach
    ///     // the await point.
    ///     bar(some_value).await;
    /// }
    /// ```
    ///
    /// Or use:
    ///
    /// ```rust,ignore
    /// use tracing::{span, Level, Instrument};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///     async move {
    ///         // This is correct! If we yield here, the span will be exited,
    ///         // and re-entered when we resume.
    ///         bar().await;
    ///     }.instrument(span) // instrument the async block with the span...
    ///     .await // ...and await it.
    /// }
    /// ```
    ///
    /// [#6353]: https://github.com/rust-lang/rust-clippy/issues/6353
    pub AWAIT_HOLDING_SPAN_GUARD,
    Warn,
    "Inside an async function, holding a Span guard while calling await"
}

const TRACING_SPAN_ENTER_GUARD: [&str; 3] = ["tracing", "span", "Entered"];
const TRACING_SPAN_ENTERED_GUARD: [&str; 3] = ["tracing", "span", "EnteredSpan"];

impl LateLintPass<'_> for AwaitHoldingSpanGuard {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ hir::Expr<'_>) {
        if let hir::ExprKind::Closure(hir::Closure {
            def_id,
            kind:
                hir::ClosureKind::Coroutine(hir::CoroutineKind::Desugared(
                    hir::CoroutineDesugaring::Async,
                    _,
                )),
            ..
        }) = expr.kind
            && let Some(coroutine_layout) = cx.tcx.mir_coroutine_witnesses(*def_id)
        {
            check_interior_types(cx, coroutine_layout);
        }
    }
}

// smoelius: As part of the upgrade to nightly-2023-10-06, `check_interior_types` was updated based
// on: https://github.com/rust-lang/rust-clippy/commit/0a2d39de2e0b87361432ae695cc84ad74d09972a
fn check_interior_types(cx: &LateContext<'_>, coroutine: &CoroutineLayout<'_>) {
    for (ty_index, ty_cause) in coroutine.field_tys.iter_enumerated() {
        if let Adt(adt, _) = ty_cause.ty.kind() {
            let await_points = || {
                let mut spans = Vec::new();
                for (variant, source_info) in coroutine.variant_source_info.iter_enumerated() {
                    if coroutine.variant_fields[variant].raw.contains(&ty_index) {
                        spans.push(source_info.span);
                    }
                }
                spans
            };
            if is_tracing_span_guard(cx, adt.did()) {
                span_lint_and_then(
                    cx,
                    AWAIT_HOLDING_SPAN_GUARD,
                    ty_cause.source_info.span,
                    "this Span guard is held across an 'await' point. Consider using the \
                     `.instrument()` combinator or the `.in_scope()` method instead",
                    |diag| {
                        diag.span_note(
                            await_points(),
                            "these are all the await points this ref is held through",
                        );
                    },
                );
            }
        }
    }
}

fn is_tracing_span_guard(cx: &LateContext<'_>, def_id: DefId) -> bool {
    match_def_path(cx, def_id, &TRACING_SPAN_ENTER_GUARD)
        || match_def_path(cx, def_id, &TRACING_SPAN_ENTERED_GUARD)
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
