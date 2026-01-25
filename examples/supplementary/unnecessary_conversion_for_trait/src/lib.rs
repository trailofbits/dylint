#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use clippy_utils::{
    diagnostics::{span_lint, span_lint_and_help, span_lint_and_sugg},
    get_parent_expr,
    source::snippet_opt,
    ty::is_copy,
};
use dylint_internal::{cargo::current_metadata, match_def_path};
use rustc_errors::Applicability;
use rustc_hir::{
    BorrowKind, Expr, ExprKind, Mutability,
    def_id::{DefId, LOCAL_CRATE},
};
use rustc_index::bit_set::DenseBitSet;
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self, ClauseKind, EarlyBinder, FnDef, FnSig, GenericArgsRef, Param, ParamTy,
    ProjectionPredicate, Ty,
    adjustment::{Adjust, Adjustment, AutoBorrow},
};
use rustc_span::symbol::{Symbol, sym};
use rustc_trait_selection::traits::{
    Obligation, ObligationCause, query::evaluate_obligation::InferCtxtExt,
};
use std::{
    collections::{BTreeSet, VecDeque},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
};

mod check_inherents;
use check_inherents::check_inherents;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for trait-behavior-preserving calls in positions where a trait implementation is
    /// expected.
    ///
    /// ### Why is this bad?
    ///
    /// Such unnecessary calls make the code more verbose and could impact performance.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls").args(["-a", "-l"].iter());
    /// let _ = Path::new("/").join(Path::new("."));
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls").args(["-a", "-l"]);
    /// let _ = Path::new("/").join(".");
    /// ```
    pub UNNECESSARY_CONVERSION_FOR_TRAIT,
    Warn,
    "unnecessary calls that preserve trait behavior",
    UnnecessaryConversionForTrait::default()
}

#[derive(Default)]
struct UnnecessaryConversionForTrait {
    callee_paths: BTreeSet<Vec<String>>,
}

const WATCHED_TRAITS: &[&[&str]] = &[
    &["alloc", "borrow", "ToOwned", "to_owned"],
    &["alloc", "string", "ToString", "to_string"],
    &["core", "borrow", "Borrow", "borrow"],
    &["core", "borrow", "BorrowMut", "borrow_mut"],
    &["core", "convert", "AsMut", "as_mut"],
    &["core", "convert", "AsRef", "as_ref"],
    &["core", "ops", "deref", "Deref", "deref"],
    &["core", "ops", "deref", "DerefMut", "deref_mut"],
];

const WATCHED_INHERENTS: &[&[&str]] = &[
    &["alloc", "slice", "<impl [T]>", "into_vec"],
    &["alloc", "slice", "<impl [T]>", "to_vec"],
    &["alloc", "str", "<impl str>", "into_boxed_bytes"],
    &["alloc", "str", "<impl str>", "into_string"],
    &["alloc", "string", "String", "as_bytes"],
    &["alloc", "string", "String", "as_mut_str"],
    &["alloc", "string", "String", "as_str"],
    &["alloc", "string", "String", "into_boxed_str"],
    &["alloc", "string", "String", "into_bytes"],
    &["alloc", "vec", "Vec", "as_mut_slice"],
    &["alloc", "vec", "Vec", "as_slice"],
    &["alloc", "vec", "Vec", "into_boxed_slice"],
    &["core", "slice", "<impl [T]>", "as_mut_slice"],
    &["core", "slice", "<impl [T]>", "as_slice"],
    &["core", "slice", "<impl [T]>", "iter"],
    &["core", "slice", "<impl [T]>", "iter_mut"],
    &["core", "str", "<impl str>", "as_bytes"],
    &["core", "str", "<impl str>", "as_str"],
    &["std", "ffi", "os_str", "OsStr", "as_encoded_bytes"],
    &["std", "ffi", "os_str", "OsStr", "as_os_str"],
    &["std", "ffi", "os_str", "OsStr", "into_os_string"],
    &["std", "ffi", "os_str", "OsStr", "new"],
    &["std", "ffi", "os_str", "OsStr", "to_os_string"],
    &["std", "ffi", "os_str", "OsString", "as_os_str"],
    &["std", "ffi", "os_str", "OsString", "into_boxed_os_str"],
    &["std", "ffi", "os_str", "OsString", "into_encoded_bytes"],
    &["std", "path", "Path", "as_mut_os_str"],
    &["std", "path", "Path", "as_os_str"],
    &["std", "path", "Path", "as_path"],
    &["std", "path", "Path", "into_path_buf"],
    &["std", "path", "Path", "iter"],
    &["std", "path", "Path", "new"],
    &["std", "path", "Path", "to_path_buf"],
    &["std", "path", "PathBuf", "as_mut_os_string"],
    &["std", "path", "PathBuf", "as_path"],
    &["std", "path", "PathBuf", "into_boxed_path"],
    &["std", "path", "PathBuf", "into_os_string"],
    &["tempfile", "dir", "TempDir", "path"],
    &["tempfile", "file", "NamedTempFile", "path"],
];

const IGNORED_INHERENTS: &[&[&str]] = &[
    &["alloc", "str", "<impl str>", "to_ascii_lowercase"],
    &["alloc", "str", "<impl str>", "to_ascii_uppercase"],
    &["alloc", "str", "<impl str>", "to_lowercase"],
    &["alloc", "str", "<impl str>", "to_uppercase"],
    &["alloc", "string", "String", "from_utf16_lossy"],
    &["alloc", "string", "String", "from_utf16be_lossy"],
    &["alloc", "string", "String", "from_utf16le_lossy"],
    &["alloc", "string", "String", "from_utf8_lossy_owned"],
    &["alloc", "string", "String", "leak"],
    &["alloc", "vec", "Vec", "const_make_global"],
    &["alloc", "vec", "Vec", "into_chunks"],
    &["alloc", "vec", "Vec", "into_flattened"],
    &["alloc", "vec", "Vec", "leak"],
    &["alloc", "vec", "Vec", "recycle"],
    &["alloc", "vec", "Vec", "spare_capacity_mut"],
    &["core", "slice", "<impl [T]>", "as_chunks_unchecked"],
    &["core", "slice", "<impl [T]>", "as_chunks_unchecked_mut"],
    &["core", "str", "<impl str>", "as_bytes_mut"],
    &["core", "str", "<impl str>", "trim"],
    &["core", "str", "<impl str>", "trim_ascii"],
    &["core", "str", "<impl str>", "trim_ascii_end"],
    &["core", "str", "<impl str>", "trim_ascii_start"],
    &["core", "str", "<impl str>", "trim_end"],
    &["core", "str", "<impl str>", "trim_left"],
    &["core", "str", "<impl str>", "trim_right"],
    &["core", "str", "<impl str>", "trim_start"],
    &["std", "ffi", "os_str", "OsStr", "to_ascii_lowercase"],
    &["std", "ffi", "os_str", "OsStr", "to_ascii_uppercase"],
    &["std", "ffi", "os_str", "OsString", "leak"],
    &["std", "path", "Path", "trim_trailing_sep"],
    &["std", "path", "PathBuf", "leak"],
];

impl<'tcx> LateLintPass<'tcx> for UnnecessaryConversionForTrait {
    #[expect(clippy::too_many_lines)]
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some((maybe_call, maybe_arg, ancestor_mutabilities)) =
            ancestor_addr_of_mutabilities(cx, expr)
            && let Some((outer_callee_def_id, outer_substs, outer_receiver, outer_args)) =
                get_callee_generic_args_and_args(cx, maybe_call)
            && let outer_args = std::iter::once(outer_receiver)
                .flatten()
                .chain(outer_args)
                .collect::<Vec<_>>()
            && let outer_fn_sig = cx
                .tcx
                .fn_sig(outer_callee_def_id)
                .skip_binder()
                .skip_binder()
            && let Some(i) = outer_args
                .iter()
                .position(|arg| arg.hir_id == maybe_arg.hir_id)
            && let Some(input) = outer_fn_sig.inputs().get(i)
            && let Param(param_ty) = input.kind()
        {
            let mut strip_unnecessary_conversions = |mut expr, mut mutabilities| {
                let mut refs_prefix = None;

                loop {
                    if let Some((inner_callee_def_id, _, inner_receiver, inner_args)) =
                        get_callee_generic_args_and_args(cx, expr)
                        && let inner_args = std::iter::once(inner_receiver)
                            .flatten()
                            .chain(inner_args)
                            .collect::<Vec<_>>()
                        && let &[maybe_boxed_inner_arg] = inner_args.as_slice()
                        && let inner_arg = peel_boxes(cx, maybe_boxed_inner_arg)
                        && let inner_arg_ty = cx.typeck_results().expr_ty(inner_arg)
                        && let adjustment_mutabilities = adjustment_mutabilities(cx, inner_arg)
                        && let new_mutabilities = [adjustment_mutabilities, mutabilities].concat()
                        && let (new_ty, new_refs_prefix) =
                            build_ty_and_refs_prefix(cx, inner_arg_ty, &new_mutabilities)
                        && inner_arg_implements_traits(
                            cx,
                            outer_callee_def_id,
                            outer_fn_sig,
                            outer_substs,
                            i,
                            *param_ty,
                            new_ty,
                        )
                    {
                        let inner_callee_path = cx.get_def_path(inner_callee_def_id);
                        if !WATCHED_TRAITS
                            .iter()
                            .chain(WATCHED_INHERENTS.iter())
                            .any(|path| match_def_path(cx, inner_callee_def_id, path))
                        {
                            if enabled("DEBUG_WATCHLIST") {
                                span_lint(
                                    cx,
                                    UNNECESSARY_CONVERSION_FOR_TRAIT,
                                    expr.span,
                                    format!("ignoring {inner_callee_path:?}"),
                                );
                            }
                            break;
                        }
                        self.callee_paths.insert(
                            inner_callee_path
                                .into_iter()
                                .map(Symbol::to_ident_string)
                                .collect(),
                        );
                        expr = inner_arg;
                        mutabilities = new_mutabilities;
                        refs_prefix = Some(new_refs_prefix);
                        continue;
                    }
                    break;
                }

                Some(expr).zip(refs_prefix)
            };

            if let Some((inner_arg, refs_prefix)) =
                strip_unnecessary_conversions(expr, ancestor_mutabilities)
            {
                let (is_bare_method_call, subject) =
                    if matches!(expr.kind, ExprKind::MethodCall(..)) {
                        (maybe_arg.hir_id == expr.hir_id, "receiver")
                    } else {
                        (false, "inner argument")
                    };
                let msg = format!("the {subject} implements the required traits");
                if is_bare_method_call && refs_prefix.is_empty() && !maybe_arg.span.from_expansion()
                {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_CONVERSION_FOR_TRAIT,
                        maybe_arg.span.with_lo(inner_arg.span.hi()),
                        msg,
                        "remove this",
                        String::new(),
                        Applicability::MachineApplicable,
                    );
                } else if maybe_arg.span.from_expansion()
                    && let Some(span) = maybe_arg.span.parent_callsite()
                {
                    // smoelius: This message could be more informative.
                    span_lint_and_help(
                        cx,
                        UNNECESSARY_CONVERSION_FOR_TRAIT,
                        span,
                        msg,
                        None,
                        "use the macro arguments directly",
                    );
                } else if let Some(snippet) = snippet_opt(cx, inner_arg.span) {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_CONVERSION_FOR_TRAIT,
                        maybe_arg.span,
                        msg,
                        "use",
                        format!("{refs_prefix}{snippet}"),
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        if enabled("COVERAGE") {
            let path = coverage_path(cx.tcx.crate_name(LOCAL_CRATE).as_str());
            // smoelius: Don't overwrite an existing file.
            if path.exists() {
                return;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            for path in &self.callee_paths {
                writeln!(file, "{path:?}").unwrap();
            }
        }

        if enabled("CHECK_INHERENTS") {
            check_inherents(cx);
        }
    }
}

#[cfg(test)]
mod sort {
    use super::*;

    #[cfg_attr(
        dylint_lib = "assert_eq_arg_misordering",
        expect(assert_eq_arg_misordering)
    )]
    #[test]
    fn watched_traits_are_sorted() {
        assert_eq!(&sort(WATCHED_TRAITS), &WATCHED_TRAITS);
    }

    #[cfg_attr(
        dylint_lib = "assert_eq_arg_misordering",
        expect(assert_eq_arg_misordering)
    )]
    #[test]
    fn watched_inherents_are_sorted() {
        assert_eq!(&sort(WATCHED_INHERENTS), &WATCHED_INHERENTS);
    }

    #[cfg_attr(
        dylint_lib = "assert_eq_arg_misordering",
        expect(assert_eq_arg_misordering)
    )]
    #[test]
    fn ignored_inherents_are_sorted() {
        assert_eq!(&sort(IGNORED_INHERENTS), &IGNORED_INHERENTS);
    }

    fn sort<'a, 'b>(items: &[&'a [&'b str]]) -> Vec<&'a [&'b str]> {
        let mut items = items.to_vec();
        items.sort_unstable();
        items
    }
}

#[cfg(test)]
mod ui {
    use super::*;
    use std::{
        env::{remove_var, set_var, var_os},
        ffi::{OsStr, OsString},
        fs::{read_to_string, remove_file, write},
        sync::Mutex,
    };
    use tempfile::tempdir;

    static MUTEX: Mutex<()> = Mutex::new(());

    #[cfg_attr(dylint_lib = "general", expect(non_thread_safe_call_in_test))]
    #[test]
    fn general() {
        let _lock = MUTEX.lock().unwrap();
        let _var = VarGuard::set("COVERAGE", "1");

        assert!(!enabled("CHECK_INHERENTS"));

        let path = coverage_path("general");
        remove_file(&path).unwrap_or_default();

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "general");

        let mut combined_watchlist = WATCHED_TRAITS
            .iter()
            .chain(WATCHED_INHERENTS.iter())
            .collect::<Vec<_>>();
        combined_watchlist.sort();

        let coverage = read_to_string(path).unwrap();
        let coverage_lines = coverage.lines().collect::<Vec<_>>();

        for (left, right) in combined_watchlist
            .iter()
            .map(|path| format!("{path:?}"))
            .zip(coverage_lines.iter())
        {
            assert_eq!(&left, right);
        }

        assert_eq!(combined_watchlist.len(), coverage_lines.len());
    }

    #[cfg_attr(dylint_lib = "general", expect(non_thread_safe_call_in_test))]
    #[test]
    fn check_inherents() {
        let _lock = MUTEX.lock().unwrap();
        let _var = VarGuard::set("CHECK_INHERENTS", "1");

        assert!(!enabled("COVERAGE"));

        let tempdir = tempdir().unwrap();

        write(tempdir.path().join("main.rs"), "fn main() {}").unwrap();

        dylint_testing::ui_test(env!("CARGO_PKG_NAME"), &tempdir);
    }

    #[test]
    fn unnecessary_to_owned() {
        let _lock = MUTEX.lock().unwrap();

        assert!(!enabled("COVERAGE"));
        assert!(!enabled("CHECK_INHERENTS"));

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "unnecessary_to_owned");
    }

    #[test]
    fn vec() {
        let _lock = MUTEX.lock().unwrap();

        assert!(!enabled("COVERAGE"));
        assert!(!enabled("CHECK_INHERENTS"));

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "vec");
    }

    // smoelius: `VarGuard` is from the following with the use of `option` added:
    // https://github.com/rust-lang/rust-clippy/blob/9cc8da222b3893bc13bc13c8827e93f8ea246854/tests/compile-test.rs
    // smoelius: Clippy dropped `VarGuard` when it switched to `ui_test`:
    // https://github.com/rust-lang/rust-clippy/commit/77d10ac63dae6ef0a691d9acd63d65de9b9bf88e

    /// Restores an env var on drop
    #[must_use]
    struct VarGuard {
        key: &'static str,
        value: Option<OsString>,
    }

    impl VarGuard {
        fn set(key: &'static str, val: impl AsRef<OsStr>) -> Self {
            let value = var_os(key);
            unsafe {
                set_var(option(key), val);
            }
            Self { key, value }
        }
    }

    impl Drop for VarGuard {
        fn drop(&mut self) {
            match self.value.as_deref() {
                None => unsafe { remove_var(option(self.key)) },
                Some(value) => unsafe { set_var(option(self.key), value) },
            }
        }
    }
}

// smoelius: `get_callee_generic_args_and_args` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/d6d530fd0b92ccec4a22e69cdebe6c4c942c8166/clippy_lints/src/methods/unnecessary_to_owned.rs#L321-L350

#[cfg_attr(
    dylint_lib = "inconsistent_qualification",
    expect(inconsistent_qualification)
)]
/// Checks whether an expression is a function or method call and, if so, returns its `DefId`,
/// `GenericArgs`, and arguments.
fn get_callee_generic_args_and_args<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(
    DefId,
    GenericArgsRef<'tcx>,
    Option<&'tcx Expr<'tcx>>,
    &'tcx [Expr<'tcx>],
)> {
    if let ExprKind::Call(callee, args) = expr.kind
        && let callee_ty = cx.typeck_results().expr_ty(callee)
        && let ty::FnDef(callee_def_id, _) = callee_ty.kind()
    {
        let generic_args = cx.typeck_results().node_args(callee.hir_id);
        return Some((*callee_def_id, generic_args, None, args));
    }
    if let ExprKind::MethodCall(_, recv, args, _) = expr.kind
        && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
    {
        let generic_args = cx.typeck_results().node_args(expr.hir_id);
        return Some((method_def_id, generic_args, Some(recv), args));
    }
    None
}

// smoelius: `inner_arg_implements_traits` is based on `needless_borrow_impl_arg_position` from:
// https://github.com/rust-lang/rust-clippy/blob/c419d0a8b538de6000226cc54a2f18a03bbd31d6/clippy_lints/src/dereference.rs#L994-L1122
fn inner_arg_implements_traits<'tcx>(
    cx: &LateContext<'tcx>,
    callee_def_id: DefId,
    fn_sig: FnSig<'tcx>,
    generic_args_with_expr_ty: GenericArgsRef<'tcx>,
    arg_index: usize,
    param_ty: ParamTy,
    new_ty: Ty<'tcx>,
) -> bool {
    let destruct_trait_def_id = cx.tcx.lang_items().destruct_trait();
    let sized_trait_def_id = cx.tcx.lang_items().sized_trait();

    let predicates = cx.tcx.param_env(callee_def_id).caller_bounds();
    let projection_predicates = predicates
        .iter()
        .filter_map(|predicate| {
            if let ClauseKind::Projection(projection_predicate) = predicate.kind().skip_binder() {
                Some(projection_predicate)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // If no traits were found, or only the `Destruct`, `Sized`, or `Any` traits were found, return.
    if predicates
        .iter()
        .filter_map(|predicate| {
            if let ClauseKind::Trait(trait_predicate) = predicate.kind().skip_binder()
                && trait_predicate.trait_ref.self_ty() == param_ty.to_ty(cx.tcx)
            {
                Some(trait_predicate.trait_ref.def_id)
            } else {
                None
            }
        })
        .all(|trait_def_id| {
            Some(trait_def_id) == destruct_trait_def_id
                || Some(trait_def_id) == sized_trait_def_id
                || cx.tcx.is_diagnostic_item(sym::Any, trait_def_id)
        })
    {
        return false;
    }

    let mut substs_with_new_ty = generic_args_with_expr_ty.to_vec();

    if !replace_types(
        cx,
        param_ty,
        new_ty,
        fn_sig,
        arg_index,
        &projection_predicates,
        &mut substs_with_new_ty,
    ) {
        return false;
    }

    predicates.iter().all(|predicate| {
        let predicate =
            EarlyBinder::bind(predicate).instantiate(cx.tcx, substs_with_new_ty.as_slice());
        let obligation = Obligation::new(cx.tcx, ObligationCause::dummy(), cx.param_env, predicate);
        cx.tcx
            .infer_ctxt()
            .build(cx.typing_mode())
            .predicate_must_hold_modulo_regions(&obligation)
    })
}

// smoelius: `replace_types` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/cf8a67d9ad81547895ec986f8bcb17e912037c38/clippy_lints/src/dereference.rs#L1295-L1349

#[cfg_attr(
    dylint_lib = "inconsistent_qualification",
    expect(inconsistent_qualification)
)]
// Iteratively replaces `param_ty` with `new_ty` in `substs`, and similarly for each resulting
// projected type that is a type parameter. Returns `false` if replacing the types would have an
// effect on the function signature beyond substituting `new_ty` for `param_ty`.
// See: https://github.com/rust-lang/rust-clippy/pull/9136#discussion_r927212757
fn replace_types<'tcx>(
    cx: &LateContext<'tcx>,
    param_ty: ParamTy,
    new_ty: Ty<'tcx>,
    fn_sig: FnSig<'tcx>,
    arg_index: usize,
    projection_predicates: &[ProjectionPredicate<'tcx>],
    substs: &mut [ty::GenericArg<'tcx>],
) -> bool {
    let mut replaced = DenseBitSet::new_empty(substs.len());

    let mut deque = VecDeque::with_capacity(substs.len());
    deque.push_back((param_ty, new_ty));

    while let Some((param_ty, new_ty)) = deque.pop_front() {
        // If `replaced.is_empty()`, then `param_ty` and `new_ty` are those initially passed in.
        if !fn_sig.inputs_and_output.iter().enumerate().all(|(i, ty)| {
            (replaced.is_empty() && i == arg_index) || !ty.contains(param_ty.to_ty(cx.tcx))
        }) {
            return false;
        }

        substs[param_ty.index as usize] = ty::GenericArg::from(new_ty);

        // The `replaced.insert(...)` check provides some protection against infinite loops.
        if replaced.insert(param_ty.index) {
            for projection_predicate in projection_predicates {
                if projection_predicate.projection_term.self_ty() == param_ty.to_ty(cx.tcx)
                    && let Some(term_ty) = projection_predicate.term.as_type()
                    && let ty::Param(term_param_ty) = term_ty.kind()
                {
                    let projection = projection_predicate
                        .projection_term
                        .with_replaced_self_ty(cx.tcx, new_ty)
                        .expect_ty(cx.tcx)
                        .to_ty(cx.tcx);

                    if let Ok(projected_ty) = cx
                        .tcx
                        .try_normalize_erasing_regions(cx.typing_env(), projection)
                        && substs[term_param_ty.index as usize]
                            != ty::GenericArg::from(projected_ty)
                    {
                        deque.push_back((*term_param_ty, projected_ty));
                    }
                }
            }
        }
    }

    true
}

fn ancestor_addr_of_mutabilities<'tcx>(
    cx: &LateContext<'tcx>,
    mut expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>, Vec<Mutability>)> {
    let mut mutabilities = Vec::new();
    while let Some(parent) = get_parent_expr(cx, expr) {
        if let ExprKind::AddrOf(BorrowKind::Ref, mutability, _) = parent.kind {
            mutabilities.push(mutability);
            expr = parent;
        } else {
            return Some((parent, expr, mutabilities));
        }
    }
    None
}

fn peel_boxes<'tcx>(cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    const BOX_NEW: [&str; 3] = ["alloc", "boxed", "box_new"];

    #[cfg_attr(dylint_lib = "supplementary", expect(commented_out_code))]
    loop {
        // smoelius: No longer necessary since: https://github.com/rust-lang/rust/pull/108471
        /* if let ExprKind::Box(inner_expr) = expr.kind {
            expr = inner_expr;
            continue;
        } */

        if let ExprKind::Call(callee, args) = expr.kind
            && let callee_ty = cx.typeck_results().expr_ty(callee)
            && let FnDef(callee_def_id, _) = callee_ty.kind()
            && match_def_path(cx, *callee_def_id, &BOX_NEW)
            && let [inner_arg] = args
        {
            expr = inner_arg;
            continue;
        }

        break;
    }

    expr
}

fn adjustment_mutabilities<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Vec<Mutability> {
    cx.typeck_results()
        .expr_adjustments(expr)
        .iter()
        .map_while(|adjustment| {
            if let Adjustment {
                kind: Adjust::Borrow(AutoBorrow::Ref(mutability)),
                target: _,
            } = adjustment
            {
                Some((*mutability).into())
            } else {
                None
            }
        })
        .collect()
}

fn build_ty_and_refs_prefix<'tcx>(
    cx: &LateContext<'tcx>,
    mut ty: Ty<'tcx>,
    mutabilities: &[Mutability],
) -> (Ty<'tcx>, String) {
    let mut refs_prefix = String::new();
    for &mutability in mutabilities {
        // smoelius: If the type is already copy, don't bother adding any more refs.
        if is_copy(cx, ty) {
            break;
        }
        ty = Ty::new_ref(cx.tcx, cx.tcx.lifetimes.re_erased, ty, mutability);
        refs_prefix = "&".to_owned() + mutability.prefix_str() + &refs_prefix;
    }
    (ty, refs_prefix)
}

#[must_use]
fn enabled(name: &str) -> bool {
    let key = option(name);
    std::env::var(key).is_ok_and(|value| value != "0")
}

fn option(name: &str) -> String {
    env!("CARGO_PKG_NAME").to_uppercase() + "_" + name
}

fn coverage_path(krate: &str) -> PathBuf {
    let metadata = current_metadata().unwrap();
    metadata
        .target_directory
        .join(krate.to_owned() + "_coverage.txt")
        .into_std_path_buf()
}
