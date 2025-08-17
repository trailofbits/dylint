#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;

use clippy_utils::diagnostics::span_lint;
use dylint_internal::{match_def_path, paths};
use rustc_hir::{QPath, Stmt, StmtKind, TyKind, def::Res};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for local variables that are [`RefCell`]s.
    ///
    /// ### Why is this bad?
    ///
    /// There is rarely a need for a locally declared `RefCell`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::cell::RefCell;
    /// let x = RefCell::<usize>::new(0);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::cell::RefCell;
    /// let mut x: usize = 0;
    /// ```
    ///
    /// [`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
    pub LOCAL_REF_CELL,
    Warn,
    "`RefCell` local variables"
}

impl<'tcx> LateLintPass<'tcx> for LocalRefCell {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        let StmtKind::Let(let_stmt) = stmt.kind else {
            return;
        };

        let ref_cell_ty = if let Some(ty) = let_stmt.ty
            && let TyKind::Path(QPath::Resolved(_, path)) = ty.kind
            && let Res::Def(_, def_id) = path.res
            && match_def_path(cx, def_id, &paths::CELL_REF_CELL)
        {
            true
        } else {
            false
        };

        let ref_cell_init = if let Some(init) = let_stmt.init
            && let ty::Adt(adt_def, _) = cx.typeck_results().expr_ty(init).kind()
            && match_def_path(cx, adt_def.did(), &paths::CELL_REF_CELL)
        {
            true
        } else {
            false
        };

        if ref_cell_ty || ref_cell_init {
            span_lint(cx, LOCAL_REF_CELL, stmt.span, "locally declared `RefCell`");
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
