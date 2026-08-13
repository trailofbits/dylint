#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    higher::VecArgs,
    macros::{find_assert_eq_args, root_macro_call_first_node},
    source::snippet_opt,
    visitors::is_const_evaluatable,
};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for invocations of `assert_eq!` or `assert_ne!` whose arguments are "non-const,
    /// const-like", which suggests they could be "actual, expected".
    ///
    /// An argument is "const-like" if it is const-evaluatable, or if it is a non-empty `vec!`
    /// invocation whose arguments are const-like. The latter allowance is needed because a `vec!`
    /// invocation allocates, and is therefore never const-evaluatable, even when its value is
    /// determined entirely by the source text. An empty `vec!` is excluded because moving one to
    /// the first position can leave its element type uninferable, e.g., `assert_eq!(x, vec![])`
    /// compiles but `assert_eq!(vec![], x)` does not.
    ///
    /// ### Why is this bad?
    ///
    /// In a long list of output, one's eyes naturally go to the last line. Hence, it should be what
    /// is unusual, i.e., the "actual" value.
    ///
    /// ### Known problems
    ///
    /// A common source of false positives is "sorted, unsorted" where the check is of the
    /// sortedness of a collection that is const-like.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # let x = 0;
    /// assert_eq!(x, 0);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # let x = 0;
    /// assert_eq!(0, x);
    /// ```
    pub ASSERT_EQ_ARG_MISORDERING,
    Warn,
    "`assert_eq!(actual, expected)`"
}

impl<'tcx> LateLintPass<'tcx> for AssertEqArgMisordering {
    // smoelius: Loosely based on `check_expr` from Clippy's `bool-assert-comparison`:
    // https://github.com/rust-lang/rust-clippy/blob/d6d530fd0b92ccec4a22e69cdebe6c4c942c8166/clippy_lints/src/bool_assert_comparison.rs#L72
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else {
            return;
        };
        let macro_name = cx.tcx.item_name(macro_call.def_id);
        if !matches!(
            macro_name.as_str(),
            "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne"
        ) {
            return;
        }
        let Some((left, right, _)) = find_assert_eq_args(cx, expr, macro_call.expn) else {
            return;
        };
        if is_const_like(cx, left) || !is_const_like(cx, right) {
            return;
        }
        // A const-like argument can be macro generated (e.g., `vec![0]`), in which case its span
        // lies within the macro's definition. Use the call sites so that the spans refer to what
        // was written.
        let span_left = left.span.source_callsite();
        let span_right = right.span.source_callsite();
        let span_comma = span_left.with_lo(span_left.hi()).with_hi(span_right.lo());
        let Some(((snippet_left, snippet_comma), snippet_right)) = snippet_opt(cx, span_left)
            .zip(snippet_opt(cx, span_comma))
            .zip(snippet_opt(cx, span_right))
        else {
            return;
        };
        span_lint_and_sugg(
            cx,
            ASSERT_EQ_ARG_MISORDERING,
            span_left.with_hi(span_right.hi()),
            r#"arguments are "non-const, const-like", which looks like "actual, expected""#,
            r#"prefer "expected, actual""#,
            format!("{snippet_right}{snippet_comma}{snippet_left}"),
            Applicability::MachineApplicable,
        );
    }
}

/// Returns true if `expr`'s value is determined entirely by the source text, i.e., if `expr` is
/// const-evaluatable or is a `vec!` invocation whose arguments are const-like.
fn is_const_like<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    match VecArgs::hir(cx, expr) {
        // An empty `vec!` says nothing about what is expected, and moving one to the first position
        // can leave its element type uninferable, e.g., `assert_eq!(x, vec![])` compiles but
        // `assert_eq!(vec![], x)` does not. This case must be checked before
        // `is_const_evaluatable`, as `vec![]` expands to a call to the `const fn` `Vec::new`.
        Some(VecArgs::Vec([])) => false,
        Some(VecArgs::Vec(elems)) => elems.iter().all(|elem| is_const_like(cx, elem)),
        Some(VecArgs::Repeat(elem, len)) => is_const_like(cx, elem) && is_const_like(cx, len),
        None => is_const_evaluatable(cx, expr),
    }
}

// smoelius: An earlier version of this lint tried to include arguments' enclosing parens. But
// problems arise when an `assert_eq!` invocation has a message with parens.
#[cfg(any())]
fn extend_to_parens(cx: &LateContext<'_>, span: Span) -> Span {
    let before = cx
        .sess()
        .source_map()
        .span_extend_to_prev_char(span, '(', true);
    let after = cx
        .sess()
        .source_map()
        .span_extend_to_next_char(span, ')', true);
    before
        .with_lo(before.lo() - BytePos(1))
        .with_hi(after.hi() + BytePos(1))
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
