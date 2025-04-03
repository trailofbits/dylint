#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg, is_expr_path_def_path, match_any_def_paths,
    source::snippet_opt,
};
use dylint_internal::paths;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::{Span, sym};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for joining of constant path components.
    ///
    /// ### Why is this bad?
    ///
    /// Such paths can be constructed from string literals using `/`, since `/` works as a path
    /// separator on both Unix and Windows (see [`std::path::Path`]).
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let _ =
    /// PathBuf::from("..").join("target")
    /// # ;
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let _ =
    /// PathBuf::from("../target")
    /// # ;
    /// ```
    ///
    /// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
    pub CONST_PATH_JOIN,
    Warn,
    "joining of constant path components"
}

enum TyOrPartialSpan {
    Ty(&'static [&'static str]),
    PartialSpan(Span),
}

impl<'tcx> LateLintPass<'tcx> for ConstPathJoin {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        let (components, ty_or_partial_span) = collect_components(cx, expr);
        if components.len() < 2 {
            return;
        }
        let path = components.join("/");
        let (span, sugg) = match ty_or_partial_span {
            TyOrPartialSpan::Ty(ty) => (expr.span, format!(r#"{}::from("{path}")"#, ty.join("::"))),
            TyOrPartialSpan::PartialSpan(partial_span) => {
                (partial_span, format!(r#".join("{path}")"#))
            }
        };
        span_lint_and_sugg(
            cx,
            CONST_PATH_JOIN,
            span,
            "path could be constructed from a string literal",
            "use",
            sugg,
            Applicability::MachineApplicable,
        );
    }
}

fn collect_components(cx: &LateContext<'_>, mut expr: &Expr<'_>) -> (Vec<String>, TyOrPartialSpan) {
    let mut components_reversed = Vec::new();
    let mut partial_span = expr.span.with_lo(expr.span.hi());
    let mut has_const_expr = false;

    loop {
        if let ExprKind::MethodCall(_, receiver, [arg], _) = expr.kind
            && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && match_any_def_paths(
                cx,
                method_def_id,
                &[&paths::CAMINO_UTF8_PATH_JOIN, &paths::PATH_JOIN],
            )
            .is_some()
        {
            expr = receiver;
            if let Some(s) = is_lit_string(cx, arg) {
                components_reversed.push(format!(r#""{}""#, s));
            } else if is_const_expr(cx, arg) {
                has_const_expr = true;
                components_reversed.push(snippet_opt(cx, arg.span).unwrap_or_default());
            }
            partial_span = partial_span.with_lo(receiver.span.hi());
            continue;
        }
        break;
    }

    let ty_or_partial_span = if let ExprKind::Call(callee, [arg]) = expr.kind
        && let ty = is_path_buf_from(cx, callee, expr)
        && (is_expr_path_def_path(cx, callee, &paths::CAMINO_UTF8_PATH_NEW)
            || is_expr_path_def_path(cx, callee, &paths::PATH_NEW)
            || ty.is_some())
    {
        if let Some(s) = is_lit_string(cx, arg) {
            components_reversed.push(format!(r#""{}""#, s));
        } else if is_const_expr(cx, arg) {
            has_const_expr = true;
            components_reversed.push(snippet_opt(cx, arg.span).unwrap_or_default());
        }
        TyOrPartialSpan::Ty(ty.unwrap_or_else(|| {
            if is_expr_path_def_path(cx, callee, &paths::CAMINO_UTF8_PATH_NEW) {
                &paths::CAMINO_UTF8_PATH_BUF
            } else {
                &paths::PATH_PATH_BUF
            }
        }))
    } else {
        TyOrPartialSpan::PartialSpan(partial_span)
    };

    components_reversed.reverse();
    if has_const_expr {
        // If we have any constant expressions, we need to use concat! macro
        let concat_args = components_reversed.iter()
            .enumerate()
            .map(|(i, s)| {
                if i > 0 {
                    format!(r#""/", {}"#, s)
                } else {
                    s.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        components_reversed = vec![format!("concat!({})", concat_args)];
    } else {
        // For pure string literals, join with "/"
        let path = components_reversed.iter()
            .map(|s| s.trim_matches('"'))
            .collect::<Vec<_>>()
            .join("/");
        components_reversed = vec![format!(r#""{}""#, path)];
    }
    (components_reversed, ty_or_partial_span)
}

fn is_path_buf_from(
    cx: &LateContext<'_>,
    callee: &Expr<'_>,
    expr: &Expr<'_>,
) -> Option<&'static [&'static str]> {
    if let Some(callee_def_id) = cx.typeck_results().type_dependent_def_id(callee.hir_id)
        && cx.tcx.is_diagnostic_item(sym::from_fn, callee_def_id)
        && let ty = cx.typeck_results().expr_ty(expr)
        && let ty::Adt(adt_def, _) = ty.kind()
    {
        let paths: &[&[&str]] = &[&paths::CAMINO_UTF8_PATH_BUF, &paths::PATH_PATH_BUF];
        match_any_def_paths(
            cx,
            adt_def.did(),
            &[&paths::CAMINO_UTF8_PATH_BUF, &paths::PATH_PATH_BUF],
        )
        .map(|i| paths[i])
    } else {
        None
    }
}

fn is_lit_string(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    if let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Str(symbol, _) = lit.node
        && snippet_opt(cx, expr.span) == Some(format!(r#""{}""#, symbol.as_str()))
    {
        Some(symbol.to_ident_string())
    } else {
        None
    }
}

fn is_const_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::Call(callee, [arg]) = expr.kind
        && let ExprKind::Path(qpath) = callee.kind
        && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
        && let Some(path) = cx.get_def_path(def_id)
        && path.last().map_or(false, |s| s == "env")
        && let ExprKind::Lit(lit) = arg.kind
        && matches!(lit.node, LitKind::Str(_, _))
    {
        true
    } else {
        false
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
