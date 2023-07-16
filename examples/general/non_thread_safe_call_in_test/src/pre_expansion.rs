use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast::{Crate, Expr, ExprKind, Item, NodeId};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::sym;

declare_lint! {
    /// ## Pre-expansion implementation
    ///
    /// ### What it does
    /// Checks for calls to non-thread-safe functions in code attributed with
    /// `#[test]` or `#[cfg(test)]`.
    ///
    /// ### Why is this bad?
    /// "When you run multiple tests, by default they run in parallel using
    /// threads" ([reference]). Calling a non-thread-safe function in one test could affect the
    /// outcome of another.
    ///
    /// ### Known problems
    /// - Synchronization is not considered, so false positives could result.
    /// - Because this is an early lint pass (in fact, a pre-expansion pass), it could flag calls to
    ///   functions that happen to have the same name as known non-thread-safe functions.
    /// - No interprocedural analysis is done, so false negatives could result.
    /// - Things like `#[cfg(any(test, ...))]` and `#[cfg(all(test, ...))]` are not considered. This
    ///   could produce both false positives and false negatives.
    ///
    /// ### Example
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
    ///     std::process::Command::new("env")
    ///         .env("KEY", "SOME_VALUE")
    ///         .status()
    ///         .unwrap();
    /// }
    /// ```
    ///
    /// [reference]: https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively
    pub NON_THREAD_SAFE_CALL_IN_TEST_PRE_EXPANSION,
    Allow,
    "non-thread-safe function calls in tests"
}

#[derive(Default)]
pub struct NonThreadSafeCallInTest {
    stack: Vec<NodeId>,
}

impl_lint_pass!(NonThreadSafeCallInTest => [NON_THREAD_SAFE_CALL_IN_TEST_PRE_EXPANSION]);

impl EarlyLintPass for NonThreadSafeCallInTest {
    fn check_crate(&mut self, cx: &EarlyContext, _crate: &Crate) {
        if !cx.sess().opts.test {
            cx.sess().warn(
                "`non_thread_safe_call_in_test` is unlikely to be effective as `--test` was not \
                 passed to rustc",
            );
        }
    }

    fn check_item(&mut self, _cx: &EarlyContext, item: &Item) {
        if self.in_test_item() || is_test_item(item) {
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
            if self.in_test_item();
            if let ExprKind::Call(callee, _) = &expr.kind;
            if let Some(path) = is_blacklisted_function(callee);
            then {
                span_lint(
                    cx,
                    NON_THREAD_SAFE_CALL_IN_TEST_PRE_EXPANSION,
                    expr.span,
                    &(format!(
                        "calling `{}` in a test could affect the outcome of other tests",
                        path.join("::")
                    )),
                );
            }
        }
    }
}

impl NonThreadSafeCallInTest {
    fn in_test_item(&self) -> bool {
        !self.stack.is_empty()
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
        let strs: Vec<&str> = path
            .segments
            .iter()
            .map(|segment| segment.ident.as_str())
            .collect();
        crate::blacklist::BLACKLIST
            .iter()
            .copied()
            .find(|path| path.ends_with(strs.as_slice()))
    } else {
        None
    }
}
