#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;

use clippy_utils::{
    diagnostics::span_lint_and_help,
    paths::{PathLookup, PathNS, lookup_path_str},
    res::MaybeResPath,
    sym, value_path,
};
use dylint_internal::paths;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for environment variables referred to with string literals.
    ///
    /// ### Why is this bad?
    ///
    /// A typo in the string literal will result in a runtime error, not a compile time error.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let _ = std::env::var("RUSTFLAGS");
    /// unsafe {
    ///     std::env::remove_var("RUSTFALGS"); // Oops
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// const RUSTFLAGS: &str = "RUSTFLAGS";
    /// let _ = std::env::var(RUSTFLAGS);
    /// unsafe {
    ///     std::env::remove_var(RUSTFLAGS);
    /// }
    /// ```
    pub ENV_LITERAL,
    Warn,
    "environment variables referred to with string literals"
}

static ENV_VAR: PathLookup = value_path!(std::env::var);

impl<'tcx> LateLintPass<'tcx> for EnvLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if let ExprKind::Call(callee, args) = expr.kind
            && let Some(def_id) = callee.basic_res().opt_def_id()
            && (lookup_path_str(cx.tcx, PathNS::Value, &paths::ENV_REMOVE_VAR.join("::"))
                == [def_id]
                || lookup_path_str(cx.tcx, PathNS::Value, &paths::ENV_SET_VAR.join("::"))
                    == [def_id]
                || ENV_VAR.matches_path(cx, callee))
            && !args.is_empty()
            && let ExprKind::Lit(lit) = &args[0].kind
            && let LitKind::Str(symbol, _) = lit.node
            && let s = symbol.to_ident_string()
            && is_upper_snake_case(&s)
        {
            span_lint_and_help(
                cx,
                ENV_LITERAL,
                args[0].span,
                "referring to an environment variable with a string literal is error prone",
                None,
                format!("define a constant `{s}` and use that instead"),
            );
        }
    }
}

fn is_upper_snake_case(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
