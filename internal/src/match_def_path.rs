// smoelius: The functions in this module were copied from:
// https://github.com/rust-lang/rust-clippy/blob/f62f26965817f2573c2649288faa489a03ed1665/clippy_utils/src/lib.rs
// They were removed from `clippy_utils` by the following PR:
// https://github.com/rust-lang/rust-clippy/pull/14705

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use rustc_hir::{Expr, def_id::DefId};
use rustc_lint::LateContext;
use rustc_span::symbol::Symbol;

// smoelius: `is_expr_path_def_path` is based on:
// https://github.com/rust-lang/rust-clippy/blob/f62f26965817f2573c2649288faa489a03ed1665/clippy_utils/src/lib.rs#L472-L477
// It has been modified to take `path_def_id` as an argument so that `dylint_internal` does not have
// to rely on `clippy_utils`.

/// If the expression is a path, resolves it to a `DefId` and checks if it matches the given path.
///
/// Please use `is_path_diagnostic_item` if the target is a diagnostic item.
pub fn is_expr_path_def_path<'tcx>(
    path_def_id: impl Fn(&LateContext<'tcx>, &Expr<'tcx>) -> Option<DefId>,
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    segments: &[&str],
) -> bool {
    path_def_id(cx, expr).is_some_and(|id| match_def_path(cx, id, segments))
}

/// Checks if the given `DefId` matches any of the paths. Returns the index of matching path, if
/// any.
///
/// Please use `tcx.get_diagnostic_name` if the targets are all diagnostic items.
pub fn match_any_def_paths(cx: &LateContext<'_>, did: DefId, paths: &[&[&str]]) -> Option<usize> {
    let search_path = cx.get_def_path(did);
    paths.iter().position(|p| {
        p.iter()
            .map(|x| Symbol::intern(x))
            .eq(search_path.iter().copied())
    })
}

/// Checks if the given `DefId` matches the path.
pub fn match_def_path(cx: &LateContext<'_>, did: DefId, syms: &[&str]) -> bool {
    // We should probably move to Symbols in Clippy as well rather than interning every time.
    let path = cx.get_def_path(did);
    syms.iter()
        .map(|x| Symbol::intern(x))
        .eq(path.iter().copied())
}
