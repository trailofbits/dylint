#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind, HirId, LangItem, MatchSource, Node};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for `?` operators embedded within a larger expression.
    ///
    /// ### Why is this bad?
    ///
    /// It can be easy to overlook the `?`. Code is more readable when a `?` is the outermost
    /// operator in an expression.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::{env::{var, VarError}, path::PathBuf};
    /// # let _: Result<PathBuf, VarError> = (|| {
    /// Ok(PathBuf::from(&var("PWD")?))
    /// # })();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::{env::{var, VarError}, path::PathBuf};
    /// # let _: Result<PathBuf, VarError> = (|| {
    /// let val = var("PWD")?;
    /// Ok(PathBuf::from(&val))
    /// # })();
    /// ```
    pub QUESTION_MARK_IN_EXPRESSION,
    Warn,
    "`?` operators embedded within an expression"
}

impl<'tcx> LateLintPass<'tcx> for QuestionMarkInExpression {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if !cx
            .tcx
            .hir_parent_iter(expr.hir_id)
            .any(|(hir_id, _)| cx.tcx.hir_span(hir_id).in_derive_expansion())
            && let ExprKind::Match(_, _, MatchSource::TryDesugar(_)) = expr.kind
            && let Some((Node::Expr(ancestor), child_hir_id)) =
                get_filtered_ancestor(cx, expr.hir_id)
            // smoelius: `Assign`, `AssignOp`, `If`, `Let`, and `Match` expressions get a pass.
            && !match ancestor.kind {
                ExprKind::Let(..) => true,
                ExprKind::If(condition, _, _) => condition.hir_id == child_hir_id,
                ExprKind::Match(scrutinee, _, _) => scrutinee.hir_id == child_hir_id,
                ExprKind::Assign(_, expr, _) | ExprKind::AssignOp(_, _, expr) => {
                    expr.hir_id == child_hir_id
                }
                _ => false,
            }
        {
            span_lint_and_help(
                cx,
                QUESTION_MARK_IN_EXPRESSION,
                expr.span,
                "using the `?` operator within an expression",
                None,
                "consider breaking this up into multiple expressions",
            );
        }
    }
}

fn get_filtered_ancestor<'hir>(
    cx: &LateContext<'hir>,
    hir_id: HirId,
) -> Option<(Node<'hir>, HirId)> {
    let mut child_hir_id = hir_id;
    for (hir_id, node) in cx.tcx.hir_parent_iter(hir_id) {
        if let Node::Expr(expr) = node {
            if matches!(
                expr.kind,
                ExprKind::Binary(_, _, _) | ExprKind::Unary(_, _) | ExprKind::DropTemps(_)
            ) {
                child_hir_id = hir_id;
                continue;
            }

            if let ExprKind::Call(callee, _) = expr.kind
                && let ExprKind::Path(qpath) = callee.kind
                && cx.tcx.qpath_is_lang_item(qpath, LangItem::IntoIterIntoIter)
            {
                child_hir_id = hir_id;
                continue;
            }
        }

        return Some((node, child_hir_id));
    }
    None
}

#[test]
fn ui_example() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "clone");
}

#[test]
fn ui_examples() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
