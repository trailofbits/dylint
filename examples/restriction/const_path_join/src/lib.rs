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
    source::snippet_opt, visitors::is_const_evaluatable,
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

#[derive(Clone, Debug)]
enum ComponentType {
    Literal(String),
    Constant(String),
    Other,
}

enum TyOrPartialSpan {
    Ty(&'static [&'static str]),
    PartialSpan(Span),
}

impl<'tcx> LateLintPass<'tcx> for ConstPathJoin {
    #[allow(clippy::too_many_lines)]
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let (components, ty_or_partial_span) = collect_components(cx, expr);
        if components.len() < 2 {
            return;
        }

        let all_literals = components
            .iter()
            .all(|c| matches!(c, ComponentType::Literal(_)));
        
        let all_const_or_literal = components
            .iter()
            .all(|c| !matches!(c, ComponentType::Other));
            
        // Only consider constants if there are multiple constants
        // If there's just one constant with string literals, treat as normal joins
        let has_multiple_constants = components
            .iter()
            .filter(|c| matches!(c, ComponentType::Constant(_)))
            .count() > 1;

        if all_literals {
            // Suggest joining into a single string literal
            let joined_path = components
                .iter()
                .map(|c| match c {
                    ComponentType::Literal(s) => s.as_str(),
                    _ => unreachable!(), // Already checked all are literals
                })
                .collect::<Vec<&str>>()
                .join("/");
            
            let (span, sugg) = match ty_or_partial_span {
                TyOrPartialSpan::Ty(ty) => (
                    expr.span,
                    format!(r#"{}::from("{joined_path}")"#, ty.join("::"))
                ),
                TyOrPartialSpan::PartialSpan(partial_span) => {
                    (partial_span, format!(r#".join("{joined_path}")"#))
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
        } else if all_const_or_literal && has_multiple_constants {
            // Suggest using concat!() only when there are multiple constant expressions
            let concat_args = components
                .iter()
                .enumerate()
                .flat_map(|(i, c)| {
                    let mut items = Vec::new();
                    match c {
                        ComponentType::Literal(s) => items.push(format!(r#""{s}""#)),
                        ComponentType::Constant(s) => items.push(s.clone()),
                        ComponentType::Other => unreachable!(),
                    }
                    // Add path separator, except for the last component
                    if i < components.len() - 1 {
                        items.push(r#""/""#.to_string());
                    }
                    items
                })
                .collect::<Vec<String>>()
                .join(", ");

            let (span, sugg) = match ty_or_partial_span {
                TyOrPartialSpan::Ty(ty) => (
                    expr.span,
                    format!(r"{}::from(concat!({concat_args}))", ty.join("::"))
                ),
                TyOrPartialSpan::PartialSpan(partial_span) => {
                    // We need to replace the entire chain of joins with a single join of the concat
                    let full_span = expr.span.with_lo(partial_span.lo());
                    (full_span, format!(r".join(concat!({concat_args}))"))
                }
            };
            
            span_lint_and_sugg(
                cx,
                CONST_PATH_JOIN,
                span, 
                "path could be constructed with concat!",
                "consider using",
                sugg,
                Applicability::MachineApplicable,
            );
        } else if all_const_or_literal {
            // For a single constant expression with string literals, treat like string literals
            let joined_path = components
                .iter()
                .map(|c| match c {
                    ComponentType::Literal(s) => s.as_str(),
                    ComponentType::Constant(_) => "..", // We know it's a path component
                    ComponentType::Other => unreachable!(), // Already checked all are const or literals
                })
                .collect::<Vec<&str>>()
                .join("/");
            
            let (span, sugg) = match ty_or_partial_span {
                TyOrPartialSpan::Ty(ty) => (
                    expr.span,
                    format!(r#"{}::from("{joined_path}")"#, ty.join("::"))
                ),
                TyOrPartialSpan::PartialSpan(partial_span) => {
                    // Use a wider span to replace all the joins
                    let full_span = expr.span.with_lo(partial_span.lo());
                    (full_span, format!(r#".join("{joined_path}")"#))
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
        // If it contains ComponentType::Other, do nothing
    }
}

fn collect_components<'tcx>(cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>) -> (Vec<ComponentType>, TyOrPartialSpan) {
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
            && let component_type = check_component_type(cx, arg)
            && !matches!(component_type, ComponentType::Other)
        {
            expr = receiver;
            components_reversed.push(component_type);
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
        && let component_type = check_component_type(cx, arg)
        && !matches!(component_type, ComponentType::Other)
    {
        components_reversed.push(component_type);
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
    if !expr.span.from_expansion()
        && let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Str(symbol, _) = lit.node
        && snippet_opt(cx, expr.span) == Some(format!(r#""{}""#, symbol.as_str()))
    {
        Some(symbol.to_ident_string())
    } else {
        None
    }
}

fn check_component_type<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> ComponentType {
    // Check if it's a direct string literal
    if let Some(s) = is_lit_string(cx, expr) {
        return ComponentType::Literal(s);
    }

    // Check if it's a concat!() or env!() macro
    if let ExprKind::Call(callee, _) = expr.kind {
        if is_expr_path_def_path(cx, callee, &["std", "concat"]) || 
           is_expr_path_def_path(cx, callee, &["std", "env"]) {
            if let Some(snippet) = snippet_opt(cx, expr.span) {
                return ComponentType::Constant(snippet);
            }
        }
    }

    // Check if it's a constant-evaluatable string expression
    if !expr.span.from_expansion()
        && is_const_evaluatable(cx, expr)
        && matches!(cx.typeck_results().expr_ty(expr).kind(), ty::Str)
    {
        if let Some(snippet) = snippet_opt(cx, expr.span) {
            return ComponentType::Constant(snippet);
        }
    }

    ComponentType::Other
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
