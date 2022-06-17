use clippy_utils::{
    diagnostics::span_lint_and_note, is_expr_path_def_path, match_def_path, path_def_id,
};
use dylint_internal::paths;
use if_chain::if_chain;
use rustc_hir::{
    def_id::{DefId, LocalDefId},
    intravisit::{walk_body, walk_expr, Visitor},
    Expr, ExprKind, Item, ItemKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_session::{declare_lint, impl_lint_pass};
use std::collections::HashSet;

declare_lint! {
    /// **What it does:** Checks for calls to non-thread-safe functions in code attributed with
    /// `#[test]`. For this lint to be effective, `--tests` must be passed to `cargo check`.
    ///
    /// **Why is this bad?** "When you run multiple tests, by default they run in parallel using
    /// threads"
    /// (https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively).
    /// Calling a non-thread-safe function in one test could affect the outcome of another.
    ///
    /// **Known problems:** Synchronization is not considered, so false positives could result.
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
    pub NON_THREAD_SAFE_CALL_IN_TEST,
    Warn,
    "non-thread-safe function calls in tests"
}

#[derive(Default)]
pub struct NonThreadSafeCallInTest {
    test_fns: Vec<DefId>,
}

impl_lint_pass!(NonThreadSafeCallInTest => [NON_THREAD_SAFE_CALL_IN_TEST]);

impl<'tcx> LateLintPass<'tcx> for NonThreadSafeCallInTest {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.find_test_fns(cx);
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // smoelius: Don't emit warnings if there are less than two tests, since at least two
        // threads are needed for a race.
        if self.test_fns.len() >= 2 && self.is_test_item(item) {
            Checker {
                cx,
                item,
                visited: HashSet::new(),
            }
            .visit_item(item);
        }
    }
}

impl NonThreadSafeCallInTest {
    fn find_test_fns<'tcx>(&mut self, cx: &LateContext<'tcx>) {
        for item_id in cx.tcx.hir().items() {
            let item = cx.tcx.hir().item(item_id);
            // smoelius:
            // https://rustc-dev-guide.rust-lang.org/test-implementation.html?highlight=testdesc#step-3-test-object-generation
            if_chain! {
                if let ItemKind::Const(ty, const_body_id) = item.kind;
                if let Some(ty_def_id) = path_def_id(cx, ty);
                if match_def_path(cx, ty_def_id, &paths::TEST_DESC_AND_FN);
                let const_body = cx.tcx.hir().body(const_body_id);
                if let ExprKind::Struct(_, fields, _) = const_body.value.kind;
                if let Some(testfn) = fields.iter().find(|field| field.ident.as_str() == "testfn");
                // smoelius: Callee is `self::test::StaticTestFn`.
                if let ExprKind::Call(_, [arg]) = testfn.expr.kind;
                if let ExprKind::Closure(_, _, closure_body_id, _, _) = arg.kind;
                let closure_body = cx.tcx.hir().body(closure_body_id);
                // smoelius: Callee is `self::test::assert_test_result`.
                if let ExprKind::Call(_, [arg]) = closure_body.value.kind;
                // smoelius: Callee is test function.
                if let ExprKind::Call(callee, _) = arg.kind;
                if let Some(callee_def_id) = path_def_id(cx, callee);
                then {
                    self.test_fns.push(callee_def_id);
                }
            }
        }
    }

    fn is_test_item(&self, item: &Item) -> bool {
        self.test_fns
            .iter()
            .any(|&def_id| item.def_id.to_def_id() == def_id)
    }
}

pub struct Checker<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    visited: HashSet<LocalDefId>,
}

impl<'cx, 'tcx> Visitor<'tcx> for Checker<'cx, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _) = &expr.kind {
            if let Some(path) = is_blacklisted_function(self.cx, callee) {
                span_lint_and_note(
                    self.cx,
                    NON_THREAD_SAFE_CALL_IN_TEST,
                    expr.span,
                    &format!(
                        "calling `{}` in a test could affect the outcome of other tests",
                        path.join("::")
                    ),
                    Some(self.item.ident.span),
                    &format!("the call is reachable from at least this test"),
                );
                return;
            } else {
                if_chain! {
                    if let Some(callee_def_id) = path_def_id(self.cx, *callee);
                    if let Some(local_def_id) = callee_def_id.as_local();
                    if !self.visited.contains(&local_def_id);
                    let _ = self.visited.insert(local_def_id);
                    let hir_id = self.cx.tcx.hir().local_def_id_to_hir_id(local_def_id);
                    if let Some(body_id) = self.cx.tcx.hir().maybe_body_owned_by(hir_id);
                    then {
                        let body = self.cx.tcx.hir().body(body_id);
                        walk_body(self, body);
                        return;
                    }
                }
            }
        }
        walk_expr(self, expr);
    }
}

fn is_blacklisted_function(cx: &LateContext<'_>, callee: &Expr) -> Option<&'static [&'static str]> {
    crate::blacklist::BLACKLIST
        .iter()
        .copied()
        .find(|path| is_expr_path_def_path(cx, callee, path))
}
