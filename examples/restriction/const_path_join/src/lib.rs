#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_sugg, is_expr_path_def_path, match_def_path};
use dylint_internal::paths;
use if_chain::if_chain;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::Span;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for joining of constant path components.
    ///
    /// ### Why is this bad?
    /// Such paths can be constructed from string literals using `/`, since `/`
    /// works as a path separator on both Unix and Windows (see [std::path::Path]).
    ///
    /// ### Example
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let _ =
    /// PathBuf::from("..").join("target")
    /// # ;
    /// ```
    /// Use instead:
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let _ =
    /// PathBuf::from("../target")
    /// # ;
    /// ```
    ///
    /// [std::path::Path]: https://doc.rust-lang.org/std/path/struct.Path.html
    pub CONST_PATH_JOIN,
    Warn,
    "joining of constant path components"
}

impl<'tcx> LateLintPass<'tcx> for ConstPathJoin {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        let (components, maybe_partial_span) = collect_components(cx, expr);
        if components.len() < 2 {
            return;
        }
        let path = components.join("/");
        let (span, sugg) = if let Some(partial_span) = maybe_partial_span {
            (partial_span, format!(r#".join("{path}")"#))
        } else {
            (expr.span, format!(r#"std::path::PathBuf::from("{path}")"#))
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

fn collect_components(cx: &LateContext<'_>, mut expr: &Expr<'_>) -> (Vec<String>, Option<Span>) {
    let mut components_reversed = Vec::new();
    let mut partial_span = expr.span.with_lo(expr.span.hi());

    #[allow(clippy::while_let_loop)]
    loop {
        if_chain! {
            if let ExprKind::MethodCall(_, receiver, [arg], _) = expr.kind;
            if let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id);
            if match_def_path(cx, method_def_id, &paths::PATH_JOIN);
            if let Some(s) = is_lit_string(arg);
            then {
                expr = receiver;
                components_reversed.push(s);
                partial_span = partial_span.with_lo(receiver.span.hi());
                continue;
            } else {
                break;
            }
        }
    }

    let maybe_partial_span = if_chain! {
        if let ExprKind::Call(callee, [arg]) = expr.kind;
        if is_expr_path_def_path(cx, callee, &paths::PATH_NEW) || is_path_buf_from(cx, callee, expr);
        if let Some(s) = is_lit_string(arg);
        then {
            components_reversed.push(s);
            None
        } else {
            Some(partial_span)
        }
    };

    components_reversed.reverse();
    (components_reversed, maybe_partial_span)
}

fn is_path_buf_from(cx: &LateContext<'_>, callee: &Expr<'_>, expr: &Expr<'_>) -> bool {
    if_chain! {
        if let Some(callee_def_id) = cx.typeck_results().type_dependent_def_id(callee.hir_id);
        if cx.tcx.lang_items().from_fn() == Some(callee_def_id);
        let ty = cx.typeck_results().expr_ty(expr);
        if let ty::Adt(adt_def, _) = ty.kind();
        if match_def_path(cx, adt_def.did(), &paths::PATH_BUF);
        then {
            true
        } else {
            false
        }
    }
}

fn is_lit_string(expr: &Expr<'_>) -> Option<String> {
    if_chain! {
        if !expr.span.from_expansion();
        if let ExprKind::Lit(lit) = &expr.kind;
        if let LitKind::Str(symbol, _) = lit.node;
        then {
            Some(symbol.to_ident_string())
        } else {
            None
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
