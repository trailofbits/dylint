#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    source::{snippet_opt, trim_span},
};
use heck::ToSnakeCase;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, HirId, Mutability, def_id::DefId};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, sym};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for an `unwrap` that could be combined with an `expect` or `unwrap` using `and_then`.
    ///
    /// ### Why is this bad?
    ///
    /// Using `and_then`s tends to produce shorter method call chains, which are easier to read and
    /// reason about.
    ///
    /// ### Known problems
    ///
    /// The lint considers only `unwrap`s in method call chains. It does not consider unwrapped
    /// values that are assigned to local variables, or assignments to local variables that are
    /// later unwrapped, for example.
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// # let toml = "".parse::<toml::Value>().unwrap();
    /// let package = toml.as_table().unwrap().get("package").unwrap();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,no_run
    /// # let toml = "".parse::<toml::Value>().unwrap();
    /// let package = toml.as_table().and_then(|map| map.get("package")).unwrap();
    /// ```
    pub COLLAPSIBLE_UNWRAP,
    Warn,
    "an `unwrap` that could be combined with an `expect` or `unwrap` using `and_then`",
    CollapsibleUnwrap::default()
}

#[derive(Default)]
struct CollapsibleUnwrap {
    visited_recvs: FxHashSet<HirId>,
}

impl<'tcx> LateLintPass<'tcx> for CollapsibleUnwrap {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some((method, recv, _, _, _)) = method_call(expr)
            && ["expect", "unwrap"].contains(&method)
            && !self.visited_recvs.contains(&recv.hir_id)
        {
            self.check(cx, recv, method == "expect");
        }
    }
}

#[derive(Clone)]
struct SpanSugg {
    span: Span,
    sugg: String,
}

impl CollapsibleUnwrap {
    fn check<'tcx>(&mut self, cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>, is_expect: bool) {
        let mut and_then_span_sugg = None;
        let mut unwrap_span_sugg = None;

        loop {
            if let Some((method, mut recv, _, _, span)) = method_call(expr)
                && let snip_span = trim_span(cx.sess().source_map(), span.with_lo(recv.span.hi()))
                && let Some(snip) = snippet_opt(cx, snip_span)
                && let Some((span_sugg, prefix)) = if method == "and_then" {
                    Some((&mut and_then_span_sugg, snip))
                } else if let Some((inner_method, inner_recv, _, _, _)) = method_call(recv)
                    && inner_method == "unwrap"
                {
                    if and_then_span_sugg.is_none() {
                        and_then_span_sugg = Some(SpanSugg {
                            span: expr.span.with_lo(expr.span.hi()),
                            sugg: String::new(),
                        });
                    }
                    unwrap_span_sugg.clone_from(&and_then_span_sugg);
                    let needs_mut = cx
                        .typeck_results()
                        .type_dependent_def_id(expr.hir_id)
                        .is_some_and(|def_id| has_ref_mut_self(cx, def_id));
                    let recv_ty = cx.typeck_results().expr_ty(recv);
                    let name = suggest_name_from_type(cx, recv_ty);
                    recv = inner_recv;
                    Some((
                        &mut unwrap_span_sugg,
                        format!(
                            ".and_then(|{}{}| {}{})",
                            if needs_mut { "mut " } else { "" },
                            name,
                            name,
                            snip
                        ),
                    ))
                } else {
                    None
                }
                && let expr_ty = cx.typeck_results().expr_ty(expr)
                && let expr_err_ty = result_err_ty(cx, expr_ty)
                && let recv_ty = cx.typeck_results().expr_ty(recv)
                && let recv_err_ty = result_err_ty(cx, recv_ty)
                && ((is_option(cx, expr_ty) && (is_option(cx, recv_ty) || recv_err_ty.is_some()))
                    || (expr_err_ty.is_some()
                        && recv_err_ty.is_some()
                        && expr_err_ty == recv_err_ty))
            {
                if let Some(span_sugg) = span_sugg {
                    span_sugg.span = trim_span(
                        cx.sess().source_map(),
                        span_sugg.span.with_lo(recv.span.hi()),
                    );
                    let needs_ok = is_option(cx, expr_ty) && recv_err_ty.is_some();
                    span_sugg.sugg = (if needs_ok { ".ok()" } else { "" }).to_owned()
                        + &prefix
                        + &span_sugg.sugg;
                }

                // smoelius: The `and_then` span and suggestion should always be at least as
                // long as the `unwrap` span and suggestion.
                if let Some(and_then_span_sugg) = and_then_span_sugg.as_mut()
                    && let Some(unwrap_span_sugg) = unwrap_span_sugg.as_ref()
                    && unwrap_span_sugg.span.lo() < and_then_span_sugg.span.lo()
                {
                    *and_then_span_sugg = unwrap_span_sugg.clone();
                }

                self.visited_recvs.insert(recv.hir_id);

                expr = recv;
            } else {
                break;
            }
        }

        if let Some(SpanSugg { span, sugg }) = unwrap_span_sugg
            && !span.is_empty()
        {
            span_lint_and_sugg(
                cx,
                COLLAPSIBLE_UNWRAP,
                span,
                if is_expect {
                    "`unwrap` that could be combined with an `expect`"
                } else {
                    "`unwrap`s that could be combined"
                },
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

// smoelius: `method_call` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/3f015a363020d3811e1f028c9ce4b0705c728289/clippy_lints/src/methods/mod.rs#L3293-L3304
/// Extracts a method call name, args, and `Span` of the method name.
fn method_call<'tcx>(
    recv: &'tcx Expr<'tcx>,
) -> Option<(&'tcx str, &'tcx Expr<'tcx>, &'tcx [Expr<'tcx>], Span, Span)> {
    if let ExprKind::MethodCall(path, receiver, args, call_span) = recv.kind
        && !args.iter().any(|e| e.span.from_expansion())
        && !receiver.span.from_expansion()
    {
        let name = path.ident.name.as_str();
        return Some((name, receiver, args, path.ident.span, call_span));
    }
    None
}

fn has_ref_mut_self(cx: &LateContext<'_>, def_id: DefId) -> bool {
    let self_ty = cx.tcx.fn_sig(def_id).skip_binder().skip_binder().inputs()[0];
    matches!(self_ty.kind(), ty::Ref(_, _, Mutability::Mut))
}

fn suggest_name_from_type(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> String {
    if let ty::Adt(adt_def, _) = ty.peel_refs().kind() {
        Some(adt_def.did())
    } else {
        None
    }
    .and_then(|def_id| cx.get_def_path(def_id).last().copied())
    .map_or_else(|| String::from("value"), |sym| sym.as_str().to_snake_case())
}

fn is_option(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
    if let ty::Adt(adt_def, _) = ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Option, adt_def.did())
    {
        true
    } else {
        false
    }
}

fn result_err_ty<'tcx>(cx: &LateContext<'tcx>, ty: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
    if let ty::Adt(adt_def, substs) = ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Result, adt_def.did())
    {
        Some(substs[1].expect_ty())
    } else {
        None
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
