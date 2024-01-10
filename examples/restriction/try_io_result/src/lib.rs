#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_help, match_def_path};
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, LangItem, MatchSource, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{GenericArgKind, Ty, TyKind};
use rustc_span::sym;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for `?` operators applied to values of type `std::io::Result`.
    ///
    /// ### Why is this bad?
    /// Returning a `std::io::Result` could mean relevant context (e.g., files or paths involved) is
    /// lost. The problem is discussed under "Verbose IO errors" in Yoshua Wuyts' [Error Handling
    /// Survey].
    ///
    /// ### Known problems
    /// No interprocedural analysis is done. So if context is added by the caller, it will go
    /// unnoticed.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fs::File;
    /// fn foo() -> std::io::Result<()> {
    ///     let _ = File::open("/dev/null")?;
    ///     Ok(())
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// # use std::fs::File;
    /// use anyhow::Context;
    /// fn foo() -> anyhow::Result<()> {
    ///     let _ = File::open("/dev/null").with_context(|| "could not open `/dev/null`")?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [Error Handling Survey]: https://blog.yoshuawuyts.com/error-handling-survey/
    pub TRY_IO_RESULT,
    Warn,
    "`?` operators applied to `std::io::Result`"
}

impl<'tcx> LateLintPass<'tcx> for TryIoResult {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) = expr.kind;
            if let ExprKind::Call(callee, [arg]) = scrutinee.kind;
            if let ExprKind::Path(path) = &callee.kind;
            if matches!(path, QPath::LangItem(LangItem::TryTraitBranch, _, _));
            if let arg_ty = cx.typeck_results().node_type(arg.hir_id);
            if is_io_result(cx, arg_ty);
            let body_owner_hir_id = cx.tcx.hir().enclosing_body_owner(expr.hir_id);
            let body_id = cx.tcx.hir().body_owned_by(body_owner_hir_id);
            let body = cx.tcx.hir().body(body_id);
            let body_ty = cx.typeck_results().expr_ty(body.value);
            if !is_io_result(cx, body_ty);
            then {
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
}

fn is_io_result(cx: &LateContext<'_>, ty: Ty) -> bool {
    if_chain! {
        if let TyKind::Adt(def, substs) = ty.kind();
        if cx.tcx.is_diagnostic_item(sym::Result, def.did());
        if let [_, generic_arg] = substs.iter().collect::<Vec<_>>().as_slice();
        if let GenericArgKind::Type(generic_arg_ty) = generic_arg.unpack();
        if let TyKind::Adt(generic_arg_def, _) = generic_arg_ty.kind();
        if match_def_path(cx, generic_arg_def.did(), &dylint_internal::paths::IO_ERROR);
        then {
            true
        } else {
            false
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
