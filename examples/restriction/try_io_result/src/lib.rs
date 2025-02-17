#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_help, match_def_path};
use rustc_hir::{Expr, ExprKind, LangItem, MatchSource, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{GenericArgKind, Ty, TyKind};
use rustc_span::sym;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for `?` operators applied to values of type `std::io::Result`.
    ///
    /// ### Why is this bad?
    ///
    /// Returning a `std::io::Result` could mean relevant context (e.g., files or paths involved) is
    /// lost. The problem is discussed under "Verbose IO errors" in Yoshua Wuyts' [Error Handling
    /// Survey].
    ///
    /// ### Known problems
    ///
    /// No interprocedural analysis is done. So if context is added by the caller, it will go
    /// unnoticed.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::fs::File;
    /// fn foo() -> anyhow::Result<()> {
    ///     let _ = File::open("/nonexistent")?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::fs::File;
    /// use anyhow::Context;
    /// fn foo() -> anyhow::Result<()> {
    ///     let _ = File::open("/nonexistent").with_context(|| "could not open `/nonexistent`")?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [Error Handling Survey]: https://blog.yoshuawuyts.com/error-handling-survey/
    pub TRY_IO_RESULT,
    Warn,
    "`?` operator applied to `std::io::Result`"
}

impl<'tcx> LateLintPass<'tcx> for TryIoResult {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) = expr.kind
            && let ExprKind::Call(callee, [arg]) = scrutinee.kind
            && let ExprKind::Path(path) = &callee.kind
            && matches!(path, QPath::LangItem(LangItem::TryTraitBranch, _))
            && let arg_ty = cx.typeck_results().node_type(arg.hir_id)
            && is_io_result(cx, arg_ty)
            && let body_owner_hir_id = cx.tcx.hir().enclosing_body_owner(expr.hir_id)
            && let body = cx.tcx.hir().body_owned_by(body_owner_hir_id)
            && let body_ty = cx.typeck_results().expr_ty(body.value)
            // smoelius: If the body's return type is `std::io::Result`, do not flag, because the
            // return type cannot carry any additional information.
            && !is_io_result(cx, body_ty)
        {
            span_lint_and_help(
                cx,
                TRY_IO_RESULT,
                expr.span,
                "returning a `std::io::Result` could discard relevant context (e.g., files or \
                 paths involved)",
                None,
                "return a type that includes relevant context",
            );
        }
    }
}

fn is_io_result(cx: &LateContext<'_>, ty: Ty) -> bool {
    if let TyKind::Adt(def, substs) = ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Result, def.did())
        && let [_, generic_arg] = substs.as_slice()
        && let GenericArgKind::Type(generic_arg_ty) = generic_arg.unpack()
        && let TyKind::Adt(generic_arg_def, _) = generic_arg_ty.kind()
        && match_def_path(cx, generic_arg_def.did(), &dylint_internal::paths::IO_ERROR)
    {
        true
    } else {
        false
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
