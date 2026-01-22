#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    paths::{PathLookup, PathNS, lookup_path_str},
    res::MaybeQPath,
    source::snippet_opt,
    value_path,
    visitors::is_const_evaluatable,
};
use dylint_internal::{match_any_def_paths, paths};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node};
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

enum ComponentType<'tcx> {
    Literal(String),
    Constant(&'tcx Expr<'tcx>),
    Other,
}

enum TyOrPartialSpan {
    Ty(&'static [&'static str]),
    PartialSpan(Span),
}

impl<'tcx> LateLintPass<'tcx> for ConstPathJoin {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if is_receiver_of_join(cx, expr) {
            return;
        }

        let (components, ty_or_partial_span) = collect_components(cx, expr);
        if components.len() < 2 {
            return;
        }

        let all_literals = components
            .iter()
            .all(|c| matches!(c, ComponentType::Literal(_)));

        let all_const_or_literal = !components.iter().any(|c| matches!(c, ComponentType::Other));

        let at_least_one_const = components
            .iter()
            .any(|c| matches!(c, ComponentType::Constant(_)));

        if all_literals {
            let joined_path = components
                .iter()
                .map(|c| match c {
                    ComponentType::Literal(s) => s.as_str(),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
                .join("/");

            let (span, sugg) = match ty_or_partial_span {
                TyOrPartialSpan::Ty(ty) => (
                    expr.span,
                    format!(r#"{}::from("{joined_path}")"#, ty.join("::")),
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
        } else if all_const_or_literal && at_least_one_const {
            let Some(concat_args) = build_concat_args(cx, &components) else {
                return;
            };

            let (span, sugg) = match ty_or_partial_span {
                TyOrPartialSpan::Ty(ty) => (
                    expr.span,
                    format!(r"{}::from(concat!({concat_args}))", ty.join("::")),
                ),
                TyOrPartialSpan::PartialSpan(partial_span) => {
                    let full_span = expr.span.with_lo(partial_span.lo());
                    (full_span, format!(r".join(concat!({concat_args}))"))
                }
            };

            span_lint_and_sugg(
                cx,
                CONST_PATH_JOIN,
                span,
                "path could be constructed with concat!",
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

static PATH_NEW: PathLookup = value_path!(std::path::Path::new);

fn is_receiver_of_join<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let Node::Expr(parent_expr) = cx.tcx.parent_hir_node(expr.hir_id)
        && let ExprKind::MethodCall(_, receiver, [arg], _) = parent_expr.kind
        && receiver.hir_id == expr.hir_id
        && let Some(method_def_id) = cx
            .typeck_results()
            .type_dependent_def_id(parent_expr.hir_id)
        && match_any_def_paths(
            cx,
            method_def_id,
            &[&paths::CAMINO_UTF8_PATH_JOIN, &paths::PATH_JOIN],
        )
        .is_some()
        && !matches!(check_component_type(cx, arg), ComponentType::Other)
    {
        return true;
    }
    false
}

fn build_concat_args<'tcx>(
    cx: &LateContext<'tcx>,
    components: &[ComponentType<'tcx>],
) -> Option<String> {
    let mut result = Vec::new();
    let mut pending_literal = String::new();

    for (i, component) in components.iter().enumerate() {
        if i != 0 {
            pending_literal.push('/');
        }
        match component {
            ComponentType::Literal(s) => {
                pending_literal.push_str(s);
            }
            ComponentType::Constant(expr) => {
                if !pending_literal.is_empty() {
                    result.push(format!(r#""{pending_literal}""#));
                    pending_literal.clear();
                }
                let snippet = snippet_opt(cx, expr.span)?;
                result.push(snippet);
            }
            ComponentType::Other => return None,
        }
    }

    if !pending_literal.is_empty() {
        result.push(format!(r#""{pending_literal}""#));
    }

    Some(result.join(", "))
}

fn collect_components<'tcx>(
    cx: &LateContext<'tcx>,
    mut expr: &'tcx Expr<'tcx>,
) -> (Vec<ComponentType<'tcx>>, TyOrPartialSpan) {
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
        && (callee.res(cx).opt_def_id().is_some_and(|def_id| {
            lookup_path_str(
                cx.tcx,
                PathNS::Value,
                &paths::CAMINO_UTF8_PATH_NEW.join("::"),
            ) == [def_id]
        }) || PATH_NEW.matches_path(cx, callee)
            || ty.is_some())
        && let component_type = check_component_type(cx, arg)
        && !matches!(component_type, ComponentType::Other)
    {
        components_reversed.push(component_type);
        TyOrPartialSpan::Ty(ty.unwrap_or_else(|| {
            if callee.res(cx).opt_def_id().is_some_and(|def_id| {
                lookup_path_str(
                    cx.tcx,
                    PathNS::Value,
                    &paths::CAMINO_UTF8_PATH_NEW.join("::"),
                ) == [def_id]
            }) {
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

fn check_component_type<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> ComponentType<'tcx> {
    if let Some(s) = is_lit_string(cx, expr) {
        return ComponentType::Literal(s);
    }

    if expr.span.from_expansion()
        && is_const_evaluatable(cx, expr)
        && cx.typeck_results().expr_ty(expr).peel_refs().is_str()
    {
        return ComponentType::Constant(expr);
    }

    ComponentType::Other
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
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
