#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{ FnDecl, intravisit::FnKind };
use rustc_lint::{ LateContext, LateLintPass };
use rustc_middle::ty::{ GenericPredicates, TyKind };
use rustc_session::{ declare_lint, declare_lint_pass };
use rustc_span::symbol::Symbol;
use rustc_span::def_id::LocalDefId;

declare_lint! {
    /// ### What it does
    /// Checks for functions that take `Iterator` trait bounds when they could use
    /// `IntoIterator` instead.
    ///
    /// ### Why is this bad?
    /// Using `IntoIterator` makes functions more flexible by allowing them to
    /// accept more types like arrays, slices, and Vec without requiring explicit 
    /// `.iter()` calls.
    ///
    /// ### Example
    /// ```rust
    /// // Bad
    /// fn process_bad<I: Iterator<Item = u32>>(iter: I) {
    ///     // ...
    /// }
    /// 
    /// // Good
    /// fn process_good<I: IntoIterator<Item = u32>>(iter: I) {
    ///     // ...
    /// }
    /// ```
    pub ARG_ITER,
    Warn,
    "functions taking `Iterator` trait bounds when `IntoIterator` would be more flexible"
}

declare_lint_pass!(ArgIter => [ARG_ITER]);

impl<'tcx> LateLintPass<'tcx> for ArgIter {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        fn_sig: &'tcx FnDecl<'tcx>,
        _: &'tcx rustc_hir::Body<'tcx>,
        span: rustc_span::Span,
        id: LocalDefId
    ) {
        // Get language items for Iterator
        let Some(iterator_def_id) = cx.tcx.lang_items().iterator_trait() else {
            return;
        };

        // Check for Iterator bounds that could be replaced with IntoIterator
        let predicates = cx.tcx.predicates_of(id);
        check_trait_predicates(cx, predicates, iterator_def_id, fn_sig, span);
    }
}

fn check_trait_predicates<'tcx>(
    cx: &LateContext<'tcx>,
    predicates: GenericPredicates<'tcx>,
    iterator_def_id: rustc_hir::def_id::DefId,
    fn_sig: &'tcx FnDecl<'tcx>,
    span: rustc_span::Span
) {
    // Keep track of which type parameters are bound by Iterator
    let mut iterator_types = Vec::new();

    // First pass: collect all type parameters bound by Iterator
    for predicate in predicates.predicates.iter() {
        if let Some(trait_pred) = predicate.0.as_trait_clause() {
            if trait_pred.def_id() == iterator_def_id {
                // Get the self type parameter name
                let self_ty = trait_pred.skip_binder().self_ty();
                if let TyKind::Param(param_ty) = self_ty.kind() {
                    iterator_types.push(param_ty.name);
                }
            }
        }
    }

    // Second pass: check if these types are used in other trait bounds
    // If they're used only for Iterator, suggest using IntoIterator instead
    for type_param in &iterator_types {
        if !is_used_in_other_bounds(predicates, *type_param, iterator_def_id) {
            // Check if any input parameter uses this type
            for input in fn_sig.inputs {
                if let Some(param_name) = find_param_type_in_ty(cx, input, *type_param) {
                    span_lint_and_help(
                        cx,
                        ARG_ITER,
                        span,
                        "parameter type has Iterator bound",
                        None,
                        format!("consider using `IntoIterator` instead of `Iterator` for parameter `{}`", param_name)
                    );
                    break;
                }
            }
        }
    }
}

// Check if the type parameter is used in bounds other than Iterator
fn is_used_in_other_bounds<'tcx>(
    predicates: GenericPredicates<'tcx>,
    type_param: Symbol,
    iterator_def_id: rustc_hir::def_id::DefId
) -> bool {
    for predicate in predicates.predicates.iter() {
        if let Some(trait_pred) = predicate.0.as_trait_clause() {
            // Skip the Iterator bound itself
            if trait_pred.def_id() == iterator_def_id {
                continue;
            }

            // Check if the type parameter is used in this bound
            let self_ty = trait_pred.skip_binder().self_ty();
            if let TyKind::Param(param_ty) = self_ty.kind() {
                if param_ty.name == type_param {
                    return true;
                }
            }
        }
    }
    false
}

// Find if a type uses the specified type parameter and return parameter name if found
fn find_param_type_in_ty<'tcx>(
    cx: &LateContext<'tcx>,
    ty: &'tcx rustc_hir::Ty<'tcx>,
    type_param: Symbol
) -> Option<String> {
    let ty = cx.typeck_results().node_type(ty.hir_id);
    match ty.kind() {
        TyKind::Param(param_ty) if param_ty.name == type_param => {
            Some(param_ty.name.to_string())
        }
        _ => None,
    }
}

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[ARG_ITER]);
    lint_store.register_late_pass(|_| Box::new(ArgIter));
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
