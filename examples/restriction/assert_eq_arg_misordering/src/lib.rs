#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    macros::{find_assert_eq_args, root_macro_call_first_node},
    source::snippet_opt,
    visitors::is_const_evaluatable,
};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for invocations of `assert_eq!` whose arguments are "non-const, const", which
    /// suggests they could be "actual, expected".
    ///
    /// ### Why is this bad?
    /// In a long list of output, one's eyes naturally go to the last line. Hence, it should be what
    /// is unusual, i.e., the "actual" value.
    ///
    /// ### Known problems
    /// A common source of false positives is "sorted, unsorted" where the check is of the
    /// sortedness of a collection that is const.
    ///
    /// ### Example
    /// ```rust
    /// # let x = 0;
    /// assert_eq!(x, 0);
    /// ```
    /// Use instead:
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
        if !matches!(macro_name.as_str(), "assert_eq" | "debug_assert_eq") {
            return;
        }
        let Some((left, right, _)) = find_assert_eq_args(cx, expr, macro_call.expn) else {
            return;
        };
        let span_comma = left.span.with_lo(left.span.hi()).with_hi(right.span.lo());
        let Some(((snippet_left, snippet_comma), snippet_right)) = snippet_opt(cx, left.span)
            .zip(snippet_opt(cx, span_comma))
            .zip(snippet_opt(cx, right.span))
        else {
            return;
        };
        if !is_const_evaluatable(cx, left) && is_const_evaluatable(cx, right) {
            span_lint_and_sugg(
                cx,
                ASSERT_EQ_ARG_MISORDERING,
                left.span.with_hi(right.span.hi()),
                r#"arguments are "non-const, const", which looks like "actual, expected""#,
                r#"prefer "expected, actual""#,
                format!("{snippet_right}{snippet_comma}{snippet_left}"),
                Applicability::MachineApplicable,
            );
        }
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
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
