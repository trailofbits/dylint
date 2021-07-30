use clippy_utils::{diagnostics::span_lint_and_help, match_def_path, paths};
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, LangItem, MatchSource, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{subst::GenericArgKind, TyKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for `?` operators applied to values of type `std::io::Result`.
    ///
    /// **Why is this bad?** Returning a `std::io::Result` could mean relevant context (e.g., files
    /// or paths involved) is lost. The problem is discussed under "Verbose IO errors" here:
    /// https://blog.yoshuawuyts.com/error-handling-survey/
    ///
    /// **Known problems:** No interprocedural analysis is done. So if context is added by the
    /// caller, it will go unnoticed.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// fn foo() -> std::io::Result<()> {
    ///     let _ = File::open("/dev/null")?;
    ///     Ok(())
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// use anyhow::Context;
    /// fn foo() -> anyhow::Result<()> {
    ///         let _ = File::open("/dev/null").with_context(|| "could not open `/dev/null`")?;
    ///     Ok(())
    /// }
    /// ```
    pub TRY_IO_RESULT,
    Warn,
    "`?` operators applied to `std::io::Result`"
}

declare_lint_pass!(TryIoResult => [TRY_IO_RESULT]);

const IO_ERROR: [&str; 4] = ["std", "io", "error", "Error"];

impl<'tcx> LateLintPass<'tcx> for TryIoResult {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Match(scrutinee, _, MatchSource::TryDesugar) = expr.kind;
            if let ExprKind::Call(callee, args) = scrutinee.kind;
            if let ExprKind::Path(path) = &callee.kind;
            if matches!(path, QPath::LangItem(LangItem::TryTraitBranch, _));
            if let Some(arg) = args.get(0);
            if let arg_ty = cx.typeck_results().node_type(arg.hir_id);
            if let TyKind::Adt(arg_def, substs) = arg_ty.kind();
            if match_def_path(cx, arg_def.did, &paths::RESULT);
            if let [_, generic_arg] = substs.iter().collect::<Vec<_>>().as_slice();
            if let GenericArgKind::Type(generic_arg_ty) = generic_arg.unpack();
            if let TyKind::Adt(generic_arg_def, _) = generic_arg_ty.kind();
            if match_def_path(cx, generic_arg_def.did, &IO_ERROR);
            then {
                span_lint_and_help(
                    cx,
                    TRY_IO_RESULT,
                    expr.span,
                    "returning a `std::io::Result` could mean relevant context (e.g., files or \
                    paths involved) is lost",
                    None,
                    "return a type that includes relevant context"
                );
            }
        }
    }
}
