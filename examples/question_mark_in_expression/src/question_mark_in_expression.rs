use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, HirId, LangItem, MatchSource, Node, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for `?` operators embedded within a larger expression.
    ///
    /// **Why is this bad?** It can be easy to overlook the `?`. Code is more readable when a `?` is
    /// the outermost operator in an expression.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # use std::{env::{var, VarError}, path::PathBuf};
    /// # let _: Result<PathBuf, VarError> = (|| {
    /// Ok(PathBuf::from(&var("PWD")?))
    /// # })();
    /// ```
    /// Use instead:
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

declare_lint_pass!(QuestionMarkInExpression => [QUESTION_MARK_IN_EXPRESSION]);

#[allow(clippy::collapsible_match)]
impl<'tcx> LateLintPass<'tcx> for QuestionMarkInExpression {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if_chain! {
            if let ExprKind::Match(_, _, MatchSource::TryDesugar) = expr.kind;
            if let Some(node_id) = get_non_into_iter_ancestor(cx, expr.hir_id);
            if let Node::Expr(ancestor) = cx.tcx.hir().get(node_id);
            then {
                // smoelius: `Let` and `Match` expressions get a pass.
                match ancestor.kind {
                    ExprKind::Let(..) | ExprKind::Match(..) => {}
                    _ => {
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
        }
    }
}

#[allow(clippy::collapsible_match)]
fn get_non_into_iter_ancestor(cx: &LateContext, hir_id: HirId) -> Option<HirId> {
    cx.tcx.hir().parent_iter(hir_id).find_map(|(hir_id, _)| {
        if_chain! {
            if let Node::Expr(expr) = cx.tcx.hir().get(hir_id);
            if let ExprKind::Call(callee, _) = expr.kind;
            if let ExprKind::Path(path) = &callee.kind;
            if let QPath::LangItem(LangItem::IntoIterIntoIter, _) = path;
            then {
                None
            } else {
                Some(hir_id)
            }
        }
    })
}
