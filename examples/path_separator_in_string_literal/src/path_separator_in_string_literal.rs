use clippy_utils::{diagnostics::span_lint_and_sugg, match_qpath, source::snippet};
use if_chain::if_chain;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for path separators (e.g., `/`) in string literals.
    ///
    /// **Why is this bad?** Path separators can vary from one OS to another. Including them in
    /// a string literal is not portable.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// PathBuf::from("../target")
    /// ```
    /// Use instead:
    /// ```rust
    /// PathBuf::from("..").join("target")
    /// ```
    pub PATH_SEPARATOR_IN_STRING_LITERAL,
    Warn,
    "path separators in string literals"
}

declare_lint_pass!(PathSeparatorInStringLiteral => [PATH_SEPARATOR_IN_STRING_LITERAL]);

const PATH_NEW: [&str; 4] = ["std", "path", "Path", "new"];
const PATH_BUF_FROM: [&str; 4] = ["std", "path", "PathBuf", "from"];

impl<'tcx> LateLintPass<'tcx> for PathSeparatorInStringLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(callee, args) = expr.kind;
            if let ExprKind::Path(path) = &callee.kind;
            if args.len() == 1;
            if let ExprKind::Lit(lit) = &args[0].kind;
            if let LitKind::Str(symbol, _) = lit.node;
            let ident = symbol.to_ident_string();
            let components = ident.split(std::path::MAIN_SEPARATOR).collect::<Vec<_>>();
            if components.len() >= 2;
            if components.iter().all(|s| !s.is_empty());
            then {
                let mut sugg = String::new();
                if match_qpath(path, &PATH_NEW) {
                    sugg = format!(r#"&{}("{}")"#, snippet(cx, callee.span, "Path::new"), components[0]);
                } else if match_qpath(path, &PATH_BUF_FROM) {
                    sugg = format!(r#"{}("{}")"#, snippet(cx, callee.span, "PathBuf::from"), components[0]);
                }
                if !sugg.is_empty() {
                    for component in &components[1..] {
                        sugg.push_str(&format!(r#".join("{}")"#, component));
                    }
                    span_lint_and_sugg(
                        cx,
                        PATH_SEPARATOR_IN_STRING_LITERAL,
                        expr.span,
                        "path separators in string literals is not portable",
                        "use",
                        sugg,
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }
}
