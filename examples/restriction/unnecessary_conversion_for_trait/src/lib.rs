#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![recursion_limit = "256"]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use clippy_utils::{
    diagnostics::{span_lint, span_lint_and_sugg},
    get_parent_expr, match_def_path,
    source::snippet_opt,
    ty::{contains_ty, is_copy},
};
use dylint_internal::cargo::current_metadata;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{
    def_id::{DefId, LOCAL_CRATE},
    BorrowKind, Expr, ExprKind, Mutability,
};
use rustc_index::bit_set::BitSet;
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self,
    adjustment::{Adjust, Adjustment, AutoBorrow},
    subst::SubstsRef,
    EarlyBinder, FnSig, Param, ParamTy, PredicateKind, ProjectionPredicate, Subst, Ty, TypeAndMut,
};
use rustc_span::symbol::{sym, Symbol};
use rustc_trait_selection::traits::{
    query::evaluate_obligation::InferCtxtExt, Obligation, ObligationCause,
};
use std::{
    collections::{BTreeSet, VecDeque},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
};

dylint_linting::impl_late_lint! {
    /// **What it does:** Checks for trait-behavior-preserving calls in positions where a trait
    /// implementation is expected.
    ///
    /// **Why is this bad?** Such unnecessary calls make the code more verbose and could impact
    /// performance.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls")
    ///     .args(["-a", "-l"].iter())
    ///     .status()
    ///     .unwrap();
    /// let _ = Path::new("/").join(Path::new("."));
    /// ```
    /// Use instead:
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls")
    ///     .args(["-a", "-l"])
    ///     .status()
    ///     .unwrap();
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

const WATCHLIST: &[&[&str]] = &[
    &["alloc", "borrow", "ToOwned", "to_owned"],
    &["alloc", "string", "String", "as_bytes"],
    &["alloc", "string", "ToString", "to_string"],
    &["core", "borrow", "Borrow", "borrow"],
    &["core", "borrow", "BorrowMut", "borrow_mut"],
    &["core", "convert", "AsMut", "as_mut"],
    &["core", "convert", "AsRef", "as_ref"],
    &["core", "ops", "deref", "Deref", "deref"],
    &["core", "ops", "deref", "DerefMut", "deref_mut"],
    &["core", "slice", "<impl [T]>", "iter"],
    &["core", "str", "<impl str>", "as_bytes"],
    &["std", "path", "Path", "new"],
    &["tempfile", "dir", "TempDir", "path"],
    &["tempfile", "file", "NamedTempFile", "path"],
];

impl<'tcx> LateLintPass<'tcx> for UnnecessaryConversionForTrait {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if_chain! {
            if let Some((maybe_call, maybe_arg, ancestor_mutabilities)) =
                ancestor_addr_of_mutabilities(cx, expr);
            if let Some((outer_callee_def_id, outer_substs, outer_args)) =
                get_callee_substs_and_args(cx, maybe_call);
            let outer_fn_sig = cx.tcx.fn_sig(outer_callee_def_id).skip_binder();
            if let Some(i) = outer_args
                .iter()
                .position(|arg| arg.hir_id == maybe_arg.hir_id);
            if let Some(input) = outer_fn_sig.inputs().get(i);
            if let Param(param_ty) = input.kind();
            if let Some((inner_callee_def_id, _, inner_args)) = get_callee_substs_and_args(cx, expr);
            if let [inner_arg] = inner_args;
            let inner_arg_ty = cx.typeck_results().expr_ty(inner_arg);
            let adjustment_mutabilities = adjustment_mutabilities(cx, inner_arg);
            let (new_ty, refs_prefix) = build_ty_and_refs_prefix(
                cx,
                inner_arg_ty,
                &[adjustment_mutabilities, ancestor_mutabilities].concat(),
            );
            if inner_arg_implements_traits(
                cx,
                outer_callee_def_id,
                outer_fn_sig,
                outer_substs,
                i,
                *param_ty,
                new_ty,
            );
            if let Some(snippet) = snippet_opt(cx, inner_arg.span);
            then {
                let inner_callee_path = cx.get_def_path(inner_callee_def_id);
                if !WATCHLIST
                    .iter()
                    .any(|path| match_def_path(cx, inner_callee_def_id, path))
                {
                    if enabled("DEBUG_WATCHLIST") {
                        span_lint(
                            cx,
                            UNNECESSARY_CONVERSION_FOR_TRAIT,
                            expr.span,
                            &format!("ignoring {:?}", inner_callee_path),
                        );
                    }
                    return;
                }
                self.callee_paths.insert(
                    inner_callee_path
                        .into_iter()
                        .map(Symbol::to_ident_string)
                        .collect(),
                );
                let (is_bare_method_call, subject) =
                    if matches!(expr.kind, ExprKind::MethodCall(..)) {
                        (maybe_arg.hir_id == expr.hir_id, "receiver")
                    } else {
                        (false, "inner argument")
                    };
                let msg = format!("the {} implements the required traits", subject);
                if is_bare_method_call && refs_prefix.is_empty() {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_CONVERSION_FOR_TRAIT,
                        maybe_arg.span.with_lo(inner_arg.span.hi()),
                        &msg,
                        "remove this",
                        String::new(),
                        Applicability::MachineApplicable,
                    );
                } else {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_CONVERSION_FOR_TRAIT,
                        maybe_arg.span,
                        &msg,
                        "use",
                        format!("{}{}", refs_prefix, snippet),
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
                writeln!(file, "{:?}", path).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        env::{remove_var, set_var},
        fs::{read_to_string, remove_file},
        sync::Mutex,
    };

    static MUTEX: Mutex<()> = Mutex::new(());

    #[cfg_attr(
        dylint_lib = "non_thread_safe_call_in_test",
        allow(non_thread_safe_call_in_test)
    )]
    #[test]
    fn general() {
        let _lock = MUTEX.lock().unwrap();

        set_var(option("COVERAGE"), "1");

        let path = coverage_path("general");
        remove_file(&path).unwrap_or_default();

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "general");

        let coverage = read_to_string(path).unwrap();
        assert_eq!(
            WATCHLIST
                .iter()
                .map(|path| format!("{:?}\n", path))
                .collect::<String>(),
            coverage
        );

        remove_var(option("COVERAGE"));
    }

    #[test]
    fn unnecessary_to_owned() {
        let _lock = MUTEX.lock().unwrap();

        assert!(!enabled("COVERAGE"));

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "unnecessary_to_owned");
    }
}

// smoelius: `get_callee_substs_and_args` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/c419d0a8b538de6000226cc54a2f18a03bbd31d6/clippy_lints/src/methods/unnecessary_to_owned.rs#L341-L365

#[cfg_attr(
    dylint_lib = "inconsistent_qualification",
    allow(inconsistent_qualification)
)]
/// Checks whether an expression is a function or method call and, if so, returns its `DefId`,
/// `Substs`, and arguments.
fn get_callee_substs_and_args<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(DefId, SubstsRef<'tcx>, &'tcx [Expr<'tcx>])> {
    if_chain! {
        if let ExprKind::Call(callee, args) = expr.kind;
        let callee_ty = cx.typeck_results().expr_ty(callee);
        if let ty::FnDef(callee_def_id, _) = callee_ty.kind();
        then {
            let substs = cx.typeck_results().node_substs(callee.hir_id);
            return Some((*callee_def_id, substs, args));
        }
    }
    if_chain! {
        if let ExprKind::MethodCall(_, args, _) = expr.kind;
        if let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id);
        then {
            let substs = cx.typeck_results().node_substs(expr.hir_id);
            return Some((method_def_id, substs, args));
        }
    }
    None
}

// smoelius: `inner_arg_implements_traits` is based on `needless_borrow_impl_arg_position` from:
// https://github.com/rust-lang/rust-clippy/blob/c419d0a8b538de6000226cc54a2f18a03bbd31d6/clippy_lints/src/dereference.rs#L994-L1122
fn inner_arg_implements_traits<'tcx>(
    cx: &LateContext<'tcx>,
    callee_def_id: DefId,
    fn_sig: FnSig<'tcx>,
    substs_with_expr_ty: SubstsRef<'tcx>,
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
            if let PredicateKind::Projection(projection_predicate) = predicate.kind().skip_binder()
            {
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
            if let PredicateKind::Trait(trait_predicate) = predicate.kind().skip_binder()
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

    let mut substs_with_new_ty = substs_with_expr_ty.to_vec();

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
        let predicate = EarlyBinder(predicate).subst(cx.tcx, &substs_with_new_ty);
        let obligation = Obligation::new(ObligationCause::dummy(), cx.param_env, predicate);
        cx.tcx
            .infer_ctxt()
            .enter(|infcx| infcx.predicate_must_hold_modulo_regions(&obligation))
    })
}

// smoelius: `replace_types` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/c419d0a8b538de6000226cc54a2f18a03bbd31d6/clippy_lints/src/dereference.rs#L1137-L1191

#[cfg_attr(
    dylint_lib = "inconsistent_qualification",
    allow(inconsistent_qualification)
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
    let mut replaced = BitSet::new_empty(substs.len());

    let mut deque = VecDeque::with_capacity(substs.len());
    deque.push_back((param_ty, new_ty));

    while let Some((param_ty, new_ty)) = deque.pop_front() {
        // If `replaced.is_empty()`, then `param_ty` and `new_ty` are those initially passed in.
        if !fn_sig.inputs_and_output.iter().enumerate().all(|(i, ty)| {
            (replaced.is_empty() && i == arg_index) || !contains_ty(ty, param_ty.to_ty(cx.tcx))
        }) {
            return false;
        }

        substs[param_ty.index as usize] = ty::GenericArg::from(new_ty);

        // The `replaced.insert(...)` check provides some protection against infinite loops.
        if replaced.insert(param_ty.index) {
            for projection_predicate in projection_predicates {
                if projection_predicate.projection_ty.self_ty() == param_ty.to_ty(cx.tcx)
                    && let ty::Term::Ty(term_ty) = projection_predicate.term
                    && let ty::Param(term_param_ty) = term_ty.kind()
                {
                    let item_def_id = projection_predicate.projection_ty.item_def_id;
                    let assoc_item = cx.tcx.associated_item(item_def_id);
                    let projection = cx.tcx
                        .mk_projection(assoc_item.def_id, cx.tcx.mk_substs_trait(new_ty, &[]));


                    if let Ok(projected_ty) = cx.tcx.try_normalize_erasing_regions(cx.param_env, projection)
                        && substs[term_param_ty.index as usize] != ty::GenericArg::from(projected_ty)
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

fn adjustment_mutabilities<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Vec<Mutability> {
    cx.typeck_results()
        .expr_adjustments(expr)
        .iter()
        .map_while(|adjustment| {
            if let Adjustment {
                kind: Adjust::Borrow(AutoBorrow::Ref(_, mutability)),
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
        ty = cx.tcx.mk_ref(
            cx.tcx.lifetimes.re_erased,
            TypeAndMut {
                ty,
                mutbl: mutability,
            },
        );
        refs_prefix = "&".to_owned() + mutability.prefix_str() + &refs_prefix;
    }
    (ty, refs_prefix)
}

#[must_use]
fn enabled(name: &str) -> bool {
    let key = option(name);
    std::env::var(key).map_or(false, |value| value != "0")
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
