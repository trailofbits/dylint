#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;

use clippy_utils::{diagnostics::span_lint_and_help, is_expr_path_def_path};
use dylint_internal::paths;
use if_chain::if_chain;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for environment variables referred to with string literals.
    ///
    /// ### Why is this bad?
    /// A typo in the string literal will result in a runtime error, not a
    /// compile time error.
    ///
    /// ### Example
    /// ```rust
    /// let _ = std::env::var("RUSTFLAGS");
    /// std::env::remove_var("RUSTFALGS"); // Oops
    /// ```
    /// Use instead:
    /// ```rust
    /// const RUSTFLAGS: &str = "RUSTFLAGS";
    /// let _ = std::env::var(RUSTFLAGS);
    /// std::env::remove_var(RUSTFLAGS);
    /// ```
    pub ENV_LITERAL,
    Warn,
    "environment variables referred to with string literals"
}

impl<'tcx> LateLintPass<'tcx> for EnvLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(callee, args) = expr.kind;
            if is_expr_path_def_path(cx, callee, &paths::ENV_REMOVE_VAR)
                || is_expr_path_def_path(cx, callee, &paths::ENV_SET_VAR)
                || is_expr_path_def_path(cx, callee, &paths::ENV_VAR);
            if !args.is_empty();
            if let ExprKind::Lit(lit) = &args[0].kind;
            if let LitKind::Str(symbol, _) = lit.node;
            let ident = symbol.to_ident_string();
            if is_upper_snake_case(&ident);
            then {
                span_lint_and_help(
                    cx,
                    ENV_LITERAL,
                    args[0].span,
                    "referring to an environment variable with a string literal is error prone",
                    None,
                    &format!("define a constant `{ident}` and use that instead"),
                );
            }
        }
    }
}

fn is_upper_snake_case(ident: &str) -> bool {
    !ident.is_empty() && ident.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
