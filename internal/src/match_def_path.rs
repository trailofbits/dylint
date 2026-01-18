// smoelius: The functions in this module were copied from:
// https://github.com/rust-lang/rust-clippy/blob/f62f26965817f2573c2649288faa489a03ed1665/clippy_utils/src/lib.rs
// They were removed from `clippy_utils` by the following PR:
// https://github.com/rust-lang/rust-clippy/pull/14705

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use rustc_hir::def_id::DefId;
use rustc_lint::LateContext;
use rustc_span::symbol::Symbol;

// smoelius: `match_any_def_paths` and `match_def_path` are from `clippy_utils`:
// https://github.com/rust-lang/rust-clippy/blob/f62f26965817f2573c2649288faa489a03ed1665/clippy_utils/src/lib.rs#L2068-L2084
// They were removed by the following commit:
// https://github.com/rust-lang/rust-clippy/commit/93bd4d893122417b9265563c037f11a158a8e37c

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
