use clippy_utils::{diagnostics::span_lint_and_help, is_expr_path_def_path};
use if_chain::if_chain;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for environment variables referred to with string literals.
    ///
    /// **Why is this bad?** A typo in the string literal will result in a runtime error, not a
    /// compile time error.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
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

declare_lint_pass!(EnvLiteral => [ENV_LITERAL]);

const REMOVE_VAR: [&str; 3] = ["std", "env", "remove_var"];
const SET_VAR: [&str; 3] = ["std", "env", "set_var"];
const VAR: [&str; 3] = ["std", "env", "var"];

impl<'tcx> LateLintPass<'tcx> for EnvLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(callee, args) = expr.kind;
            if is_expr_path_def_path(cx, callee, &REMOVE_VAR) || is_expr_path_def_path(cx, callee, &SET_VAR) || is_expr_path_def_path(cx, callee, &VAR);
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
                    &format!("define a constant `{}` and use that instead", ident),
                );
            }
        }
    }
}

fn is_upper_snake_case(ident: &str) -> bool {
    !ident.is_empty() && ident.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}
