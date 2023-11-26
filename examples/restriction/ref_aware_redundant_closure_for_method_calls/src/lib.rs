#![feature(rustc_private)]
#![allow(unused_imports)]
#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(
    dylint_lib = "inconsistent_qualification",
    allow(inconsistent_qualification)
)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::higher::VecArgs;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::{implements_trait, is_type_diagnostic_item};
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::{higher, is_adjusted, path_to_local, path_to_local_id};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::{Closure, Expr, ExprKind, Param, PatKind, Unsafety};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::adjustment::{Adjust, Adjustment, AutoBorrow};
use rustc_middle::ty::binding::BindingMode;
use rustc_middle::ty::{self, EarlyBinder, GenericArgsRef, Ty, TypeVisitable, TypeVisitableExt};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

use clippy_utils::{get_parent_expr, source::trim_span};
use rustc_lint::LintContext;
use rustc_middle::ty::adjustment::{AutoBorrowMutability, OverloadedDeref};

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// This is essentially a ref-aware fork of Clippy's [`redundant_closure_for_method_calls`]
    /// lint. It suggests to remove a closure when made possible by a use of `as_ref`, `as_mut`,
    /// `as_deref`, or `as_deref_mut`.
    ///
    /// ### Known problems
    /// Currently works only for [`Option`]s.
    ///
    /// ### Example
    /// ```rust
    /// Some(String::from("a")).map(|s| s.is_empty());
    /// Some(String::from("a")).map(|s| s.to_uppercase());
    /// ```
    /// Use instead:
    /// ```rust
    /// Some(String::from("a")).as_ref().map(String::is_empty);
    /// Some(String::from("a")).as_deref().map(str::to_uppercase);
    /// ```
    ///
    /// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
    /// [`redundant_closure_for_method_calls`]: https://rust-lang.github.io/rust-clippy/master/#redundant_closure_for_method_calls
    pub REF_AWARE_REDUNDANT_CLOSURE_FOR_METHOD_CALLS,
    Warn,
    "a ref-aware fork of `redundant_closure_for_method_calls`"
}

impl<'tcx> LateLintPass<'tcx> for RefAwareRedundantClosureForMethodCalls {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }
        let body = match expr.kind {
            ExprKind::Closure(&Closure { body, .. }) => cx.tcx.hir().body(body),
            _ => return,
        };

        #[cfg(any())]
        if body.value.span.from_expansion() {
            if body.params.is_empty() {
                if let Some(VecArgs::Vec(&[])) = higher::VecArgs::hir(cx, body.value) {
                    // replace `|| vec![]` with `Vec::new`
                    span_lint_and_sugg(
                        cx,
                        REDUNDANT_CLOSURE,
                        expr.span,
                        "redundant closure",
                        "replace the closure with `Vec::new`",
                        "std::vec::Vec::new".into(),
                        Applicability::MachineApplicable,
                    );
                }
            }
            // skip `foo(|| macro!())`
            return;
        }

        let closure_ty = cx.typeck_results().expr_ty(expr);

        #[cfg(any())]
        if_chain!(
            if !is_adjusted(cx, body.value);
            if let ExprKind::Call(callee, args) = body.value.kind;
            if let ExprKind::Path(_) = callee.kind;
            if check_inputs(cx, body.params, None, args);
            let callee_ty = cx.typeck_results().expr_ty_adjusted(callee);
            let call_ty = cx.typeck_results().type_dependent_def_id(body.value.hir_id)
                .map_or(callee_ty, |id| cx.tcx.type_of(id));
            if check_sig(cx, closure_ty, call_ty);
            let substs = cx.typeck_results().node_substs(callee.hir_id);
            // This fixes some false positives that I don't entirely understand
            if substs.is_empty() || !cx.typeck_results().expr_ty(expr).has_late_bound_regions();
            // A type param function ref like `T::f` is not 'static, however
            // it is if cast like `T::f as fn()`. This seems like a rustc bug.
            if !substs.types().any(|t| matches!(t.kind(), ty::Param(_)));
            let callee_ty_unadjusted = cx.typeck_results().expr_ty(callee).peel_refs();
            if !is_type_diagnostic_item(cx, callee_ty_unadjusted, sym::Arc);
            if !is_type_diagnostic_item(cx, callee_ty_unadjusted, sym::Rc);
            if let ty::Closure(_, substs) = *closure_ty.kind();
            then {
                span_lint_and_then(cx, REDUNDANT_CLOSURE, expr.span, "redundant closure", |diag| {
                    if let Some(mut snippet) = snippet_opt(cx, callee.span) {
                        if let Some(fn_mut_id) = cx.tcx.lang_items().fn_mut_trait()
                            && let args = cx.tcx.erase_late_bound_regions(substs.as_closure().sig()).inputs()
                            && implements_trait(
                                   cx,
                                   callee_ty.peel_refs(),
                                   fn_mut_id,
                                   &args.iter().copied().map(Into::into).collect::<Vec<_>>(),
                               )
                            && path_to_local(callee).map_or(false, |l| local_used_after_expr(cx, l, expr))
                        {
                                // Mutable closure is used after current expr; we cannot consume it.
                                snippet = format!("&mut {snippet}");
                        }
                        diag.span_suggestion(
                            expr.span,
                            "replace the closure with the function itself",
                            snippet,
                            Applicability::MachineApplicable,
                        );
                    }
                });
            }
        );

        if_chain!(
            if !is_adjusted(cx, body.value);
            if let ExprKind::MethodCall(path, receiver, args, _) = body.value.kind;
            if let Some(method_name) = check_inputs(cx, body.params, Some(receiver), args);
            if let Some(parent_expr) = get_parent_expr(cx, expr);
            if let ExprKind::MethodCall(parent_path, parent_receiver, _, span) = parent_expr.kind;
            let parent_receiver_ty = cx.typeck_results().expr_ty(parent_receiver);
            if is_type_diagnostic_item(cx, parent_receiver_ty, sym::Option);
            let method_def_id = cx.typeck_results().type_dependent_def_id(body.value.hir_id).unwrap();
            let generic_args = cx.typeck_results().node_args(body.value.hir_id);
            let call_ty = cx.tcx.type_of(method_def_id).instantiate(cx.tcx, generic_args);
            if check_sig(cx, closure_ty, call_ty);
            then {
                let parent_method_call_span = trim_span(
                    cx.sess().source_map(),
                    span.with_lo(parent_receiver.span.hi()),
                );
                span_lint_and_then(cx, REF_AWARE_REDUNDANT_CLOSURE_FOR_METHOD_CALLS, parent_method_call_span, "redundant closure", |diag| {
                let name = get_ufcs_type_name(cx, method_def_id, generic_args);
                    diag.span_suggestion(
                        parent_method_call_span,
                        "replace the closure with the method itself",
                        format!(
                            ".{method_name}().{}({name}::{})",
                            parent_path.ident.name,
                            path.ident.name
                        ),
                        Applicability::MachineApplicable,
                    );
                })
            }
        );
    }
}

fn check_inputs(
    cx: &LateContext<'_>,
    params: &[Param<'_>],
    receiver: Option<&Expr<'_>>,
    call_args: &[Expr<'_>],
) -> Option<&'static str> {
    if receiver.map_or(params.len() != call_args.len(), |_| {
        params.len() != call_args.len() + 1
    }) {
        return None;
    }
    let binding_modes = cx.typeck_results().pat_binding_modes();
    let check_inputs = |param: &Param<'_>, arg| {
        match param.pat.kind {
            PatKind::Binding(_, id, ..) if path_to_local_id(arg, id) => {}
            _ => return None,
        }
        // checks that parameters are not bound as `ref` or `ref mut`
        if let Some(BindingMode::BindByReference(_)) = binding_modes.get(param.pat.hir_id) {
            return None;
        }

        match *cx.typeck_results().expr_adjustments(arg) {
            /* [] => true,
            [
                Adjustment {
                    kind: Adjust::Deref(None),
                    ..
                },
                Adjustment {
                    kind: Adjust::Borrow(AutoBorrow::Ref(_, mu2)),
                    ..
                },
            ] => {
                // re-borrow with the same mutability is allowed
                let ty = cx.typeck_results().expr_ty(arg);
                matches!(*ty.kind(), ty::Ref(.., mu1) if mu1 == mu2.into())
            } */
            [Adjustment {
                kind: Adjust::Borrow(AutoBorrow::Ref(_, mutability)),
                ..
            }] => Some(match mutability {
                AutoBorrowMutability::Mut { .. } => "as_mut",
                AutoBorrowMutability::Not => "as_ref",
            }),
            [Adjustment {
                kind: Adjust::Deref(Some(OverloadedDeref { .. })),
                ..
            }, Adjustment {
                kind: Adjust::Borrow(AutoBorrow::Ref(_, mutability)),
                ..
            }] => Some(match mutability {
                AutoBorrowMutability::Mut { .. } => "as_deref_mut",
                AutoBorrowMutability::Not => "as_deref",
            }),
            _ => None,
        }
    };
    let adjusts = std::iter::zip(params, receiver.into_iter().chain(call_args.iter()))
        .map(|(param, arg)| check_inputs(param, arg))
        .collect::<Vec<_>>();
    if let [method_name] = adjusts.as_slice() {
        *method_name
    } else {
        None
    }
}

fn check_sig<'tcx>(cx: &LateContext<'tcx>, closure_ty: Ty<'tcx>, call_ty: Ty<'tcx>) -> bool {
    let call_sig = call_ty.fn_sig(cx.tcx);
    if call_sig.unsafety() == Unsafety::Unsafe {
        return false;
    }
    if !closure_ty.has_bound_regions() {
        return true;
    }
    let ty::Closure(_, generic_args) = closure_ty.kind() else {
        return false;
    };
    let closure_sig = cx
        .tcx
        .signature_unclosure(generic_args.as_closure().sig(), Unsafety::Normal);
    cx.tcx.erase_late_bound_regions(closure_sig) == cx.tcx.erase_late_bound_regions(call_sig)
}

fn get_ufcs_type_name<'tcx>(
    cx: &LateContext<'tcx>,
    method_def_id: DefId,
    generic_args: GenericArgsRef<'tcx>,
) -> String {
    let assoc_item = cx.tcx.associated_item(method_def_id);
    let def_id = assoc_item.container_id(cx.tcx);
    match assoc_item.container {
        ty::TraitContainer => cx.tcx.def_path_str(def_id),
        ty::ImplContainer => {
            let ty = cx.tcx.type_of(def_id).skip_binder();
            match ty.kind() {
                ty::Adt(adt, _) => cx.tcx.def_path_str(adt.did()),
                ty::Array(..)
                | ty::Dynamic(..)
                | ty::Never
                | ty::RawPtr(_)
                | ty::Ref(..)
                | ty::Slice(_)
                | ty::Tuple(_) => {
                    format!(
                        "<{}>",
                        EarlyBinder::bind(ty).instantiate(cx.tcx, generic_args)
                    )
                }
                _ => ty.to_string(),
            }
        }
    }
}

#[test]
fn ui_eta() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "eta");
}

#[test]
fn ui_ref_aware() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ref_aware");
}
