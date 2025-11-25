#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;

use clippy_utils::{diagnostics::span_lint_and_sugg, expr_or_init, source::snippet_with_applicability};
use dylint_internal::{match_any_def_paths, paths};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for calling `.path().file_name()` on a `DirEntry` when `.file_name()` can be
    /// called directly.
    ///
    /// ### Why is this bad?
    ///
    /// - For `std::fs::DirEntry`: calling `.path()` allocates a `PathBuf`, which is unnecessary
    ///   when you only need the file name. Additionally, `DirEntry::file_name()` returns an
    ///   `OsString` while `Path::file_name()` returns `Option<&OsStr>` (a more complicated type).
    /// - For `walkdir::DirEntry`: calling `.path().file_name()` returns `Option<&OsStr>` while
    ///   `.file_name()` directly returns `&OsStr` (a simpler type).
    ///
    /// ### Example
    ///
    /// ```rust
    /// use std::fs;
    ///
    /// for entry in fs::read_dir(".").unwrap() {
    ///     let entry = entry.unwrap();
    ///     let name = entry.path().file_name();
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// use std::fs;
    ///
    /// for entry in fs::read_dir(".").unwrap() {
    ///     let entry = entry.unwrap();
    ///     let name = entry.file_name();
    /// }
    /// ```
    pub DIR_ENTRY_PATH_FILE_NAME,
    Warn,
    "calling `.path().file_name()` on a DirEntry"
}

impl<'tcx> LateLintPass<'tcx> for DirEntryPathFileName {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        // Check if this is a .file_name() call with no arguments
        if let ExprKind::MethodCall(file_name_path, file_name_recv, [], _) = expr.kind
            && file_name_path.ident.name.as_str() == "file_name"
            && !expr.span.from_expansion()
        {
            // Use expr_or_init to follow variable assignments
            let actual_recv = expr_or_init(cx, file_name_recv);

            // Check if the receiver (or its initializer) is a .path() call
            if let ExprKind::MethodCall(path_path, path_recv, [], _) = actual_recv.kind
                && path_path.ident.name.as_str() == "path"
            {
                // Check type of the path() receiver - need to peel off references
                let recv_ty = cx.typeck_results().expr_ty(path_recv).peel_refs();
                if is_dir_entry_type(cx, recv_ty) {
                    emit_lint(cx, expr, path_recv);
                }
            }
        }
    }
}

fn is_dir_entry_type(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
    if let ty::Adt(adt_def, _) = ty.kind() {
        match_any_def_paths(
            cx,
            adt_def.did(),
            &[&paths::FS_DIR_ENTRY, &paths::WALKDIR_DIR_ENTRY],
        )
        .is_some()
    } else {
        false
    }
}

fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, path_recv: &Expr<'_>) {
    let mut applicability = Applicability::MachineApplicable;
    let receiver_snippet = snippet_with_applicability(cx, path_recv.span, "..", &mut applicability);

    span_lint_and_sugg(
        cx,
        DIR_ENTRY_PATH_FILE_NAME,
        expr.span,
        "file name can be obtained more directly from DirEntry",
        "try",
        format!("{receiver_snippet}.file_name()"),
        applicability,
    );
}

#[test]
fn ui_test() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui_test");
}
