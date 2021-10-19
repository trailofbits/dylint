use clippy_utils::diagnostics::span_lint;
use dylint_internal::path;
use if_chain::if_chain;
use rustc_ast::{Expr, ExprKind, Item, NodeId};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{sym, symbol::SymbolStr};

declare_lint! {
    /// **What it does:** Checks for use of nonreentrant functions in code attributed with `#[test]`
    /// or `#[cfg(test)]`.
    ///
    /// **Why is this bad?** "When you run multiple tests, by default they run in parallel using
    /// threads"
    /// (https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively).
    /// Calling a nonreentrant function in one test could affect the outcome of another.
    ///
    /// **Known problems:**
    /// * Synchronization is not considered, so false positives could result.
    /// * Because this is an early lint pass (in fact, a pre-expansion pass), it could flag calls to
    ///   functions that happen to have the same name as known nonreentrant functions.
    /// * Things like `#[cfg(any(test, ...))]` and `#[cfg(all(test, ...))]` are not considered. This
    ///   could produce both false positives and false negatives.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// #[test]
    /// fn set_var() {
    ///     std::env::set_var("KEY", "SOME_VALUE");
    ///     std::process::Command::new("env").status().unwrap();
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[test]
    /// fn set_var() {
    ///    std::process::Command::new("env")
    ///        .env("KEY", "SOME_VALUE")
    ///        .status()
    ///        .unwrap();
    /// }
    /// ```
    pub NONREENTRANT_FUNCTION_IN_TEST,
    Warn,
    "nonreentrant functions in tests"
}

#[derive(Default)]
pub struct NonreentrantFunctionInTest {
    stack: Vec<NodeId>,
}

impl_lint_pass!(NonreentrantFunctionInTest => [NONREENTRANT_FUNCTION_IN_TEST]);

const BLACKLIST: &[&[&str]] = &[
    &path::ENV_REMOVE_VAR,
    &path::ENV_SET_CURRENT_DIR,
    &path::ENV_SET_VAR,
];

impl EarlyLintPass for NonreentrantFunctionInTest {
    fn check_item(&mut self, _cx: &EarlyContext, item: &Item) {
        if !self.stack.is_empty() || is_test_item(item) {
            self.stack.push(item.id);
        }
    }

    fn check_item_post(&mut self, _cx: &EarlyContext, item: &Item) {
        if let Some(node_id) = self.stack.pop() {
            assert_eq!(node_id, item.id);
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if_chain! {
            if !self.stack.is_empty();
            if let ExprKind::Call(callee, _) = &expr.kind;
            if let Some(path) = is_blacklisted_function(&*callee);
            then {
                span_lint(
                    cx,
                    NONREENTRANT_FUNCTION_IN_TEST,
                    expr.span,
                    &(format!("calling `{}` in a test could affect the outcome of other tests", path.join("::"))),
                );
            }
        }
    }
}

fn is_test_item(item: &Item) -> bool {
    item.attrs.iter().any(|attr| {
        if attr.has_name(sym::test) {
            true
        } else {
            if_chain! {
                if attr.has_name(sym::cfg);
                if let Some(items) = attr.meta_item_list();
                if let [item] = items.as_slice();
                if let Some(feature_item) = item.meta_item();
                if feature_item.has_name(sym::test);
                then {
                    true
                } else {
                    false
                }
            }
        }
    })
}

fn is_blacklisted_function(callee: &Expr) -> Option<&'static [&'static str]> {
    if let ExprKind::Path(None, path) = &callee.kind {
        let symbols: Vec<SymbolStr> = path
            .segments
            .iter()
            .map(|segment| segment.ident.as_str())
            .collect();
        let strs: Vec<&str> = symbols.iter().map(|symbol| &**symbol).collect();
        BLACKLIST
            .iter()
            .copied()
            .find(|path| path.ends_with(strs.as_slice()))
    } else {
        None
    }
}
