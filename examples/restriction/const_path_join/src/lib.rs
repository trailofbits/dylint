#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    path_def_id,
    paths::{PathLookup, PathNS},
    source::snippet_opt,
    value_path,
};
use dylint_internal::{is_expr_path_def_path, match_any_def_paths, paths};
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
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
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

static PATH_NEW: PathLookup = value_path!(std::path::Path::new);

fn collect_components<'tcx>(
    cx: &LateContext<'tcx>,
    mut expr: &Expr<'tcx>,
) -> (Vec<String>, TyOrPartialSpan) {
    let mut components_reversed = Vec::new();
    let mut partial_span = expr.span.with_lo(expr.span.hi());

    loop {
        if let ExprKind::MethodCall(_, receiver, [arg], _) = expr.kind
            && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && match_any_def_paths(
                cx,
                method_def_id,
                &[&paths::CAMINO_UTF8_PATH_JOIN, &paths::PATH_JOIN],
            )
            .is_some()
            && let Some(s) = is_lit_string(cx, arg)
        {
            expr = receiver;
            components_reversed.push(s);
            partial_span = partial_span.with_lo(receiver.span.hi());
            continue;
        }
        break;
    }

    let ty_or_partial_span = if let ExprKind::Call(callee, [arg]) = expr.kind
        && let ty = is_path_buf_from(cx, callee, expr)
        && (is_expr_path_def_path(path_def_id, cx, callee, &paths::CAMINO_UTF8_PATH_NEW)
            || PATH_NEW.matches_path(cx, callee)
            || ty.is_some())
        && let Some(s) = is_lit_string(cx, arg)
    {
        components_reversed.push(s);
        TyOrPartialSpan::Ty(ty.unwrap_or_else(|| {
            if is_expr_path_def_path(path_def_id, cx, callee, &paths::CAMINO_UTF8_PATH_NEW) {
                &paths::CAMINO_UTF8_PATH_BUF
            } else {
                &paths::PATH_PATH_BUF
            }
        }))
    } else {
        TyOrPartialSpan::PartialSpan(partial_span)
    };

    components_reversed.reverse();
    (components_reversed, ty_or_partial_span)
}

fn is_lit_string(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    if !expr.span.from_expansion()
        && let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Str(symbol, _) = lit.node
        // smoelius: I don't think the next line should be necessary. But following the upgrade to
        // nightly-2023-08-24, `expr.span.from_expansion()` above started returning false for
        // `env!(...)`.
        && snippet_opt(cx, expr.span) == Some(format!(r#""{}""#, symbol.as_str()))
    {
        Some(symbol.to_ident_string())
    } else {
        None
    }
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

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
