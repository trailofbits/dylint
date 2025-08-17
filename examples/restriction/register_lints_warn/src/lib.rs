#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dylint_internal::match_def_path;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for calls to `rustc_errors::DiagCtxtHandle::warn` from within a `register_lints`
    /// function.
    ///
    /// ### Why is this bad?
    ///
    /// Dylint lists a library's lints by calling the library's `register_lints` function and
    /// comparing the lints that are registered before and after the call. If the library's
    /// `register_lints` functions emits warnings, they will be emitted when a user tries to list
    /// the library's lints.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_driver;
    /// # extern crate rustc_lint;
    /// # extern crate rustc_session;
    /// pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    /// #   let condition = true;
    ///     if condition {
    ///         sess.dcx().warn("something bad happened");
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_driver;
    /// # extern crate rustc_lint;
    /// # use rustc_lint::LintContext;
    /// # struct LintPass;
    /// # impl rustc_lint::LintPass for LintPass {
    /// #     fn name(&self) -> &'static str {
    /// #         "lint_pass"
    /// #     }
    /// #     fn get_lints(&self) -> Vec<&'static rustc_lint::Lint> {
    /// #         Vec::new()
    /// #     }
    /// # }
    /// impl<'tcx> rustc_lint::LateLintPass<'tcx> for LintPass {
    ///     fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
    /// #       let condition = true;
    ///         if condition {
    ///             cx.sess().dcx().warn("something bad happened");
    ///         }
    ///     }
    /// }
    /// ```
    pub REGISTER_LINTS_WARN,
    Warn,
    "calls to `rustc_errors::DiagCtxtHandle::warn` in a `register_lints` function"
}

impl<'tcx> LateLintPass<'tcx> for RegisterLintsWarn {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if matches!(expr.kind, ExprKind::MethodCall(..))
            && let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && match_def_path(cx, def_id, &["rustc_errors", "DiagCtxtHandle", "warn"])
            && let local_def_id = cx.tcx.hir_enclosing_body_owner(expr.hir_id)
            && let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id)
            && let Some(name) = cx.tcx.hir_opt_name(hir_id)
            && name.as_str() == "register_lints"
        {
            span_lint(
                cx,
                REGISTER_LINTS_WARN,
                expr.span,
                "call to `rustc_errors::DiagCtxtHandle::warn` from within a `register_lints` function",
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
