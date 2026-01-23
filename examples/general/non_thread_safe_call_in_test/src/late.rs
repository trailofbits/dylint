use clippy_utils::{
    diagnostics::span_lint_and_note,
    res::{MaybeDef, MaybeQPath, MaybeResPath},
};
use dylint_internal::{match_def_path, paths};
use rustc_ast::ast::LitKind;
use rustc_hir::{
    Closure, ConstItemRhs, Expr, ExprKind, HirId, Item, ItemKind, Node,
    def_id::{DefId, LocalDefId},
    intravisit::{Visitor, walk_body, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::hir::nested_filter;
use rustc_session::{declare_lint, impl_lint_pass};
use std::collections::HashSet;

declare_lint! {
    /// ### What it does
    ///
    /// Checks for calls to non-thread-safe functions in code attributed with
    /// `#[test]`. For this lint to be effective, `--tests` must be passed to `cargo check`.
    ///
    /// ### Why is this bad?
    ///
    /// "When you run multiple tests, by default they run in parallel using
    /// threads" ([reference]). Calling a non-thread-safe function in one test could affect the
    /// outcome of another.
    ///
    /// ### Known problems
    ///
    /// - Synchronization is not considered, so false positives could result.
    /// - Tries to flag uses of `std::process::Command::new("cargo").arg("run")`, but does not track
    ///   values. So false negatives will result if the `Command::new("cargo")` is not
    ///   `Command::arg("run")`'s receiver.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[test]
    /// fn set_var() {
    ///     std::env::set_var("KEY", "SOME_VALUE");
    ///     std::process::Command::new("env").status().unwrap();
    /// }
    /// ```
    ///
    /// Use instead:
    ///
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
    pub NON_THREAD_SAFE_CALL_IN_TEST,
    Warn,
    "non-thread-safe function calls in tests"
}

#[derive(Default)]
pub struct NonThreadSafeCallInTest {
    test_fns: Vec<DefId>,
    visited_calls: HashSet<HirId>,
}

impl_lint_pass!(NonThreadSafeCallInTest => [NON_THREAD_SAFE_CALL_IN_TEST]);

impl<'tcx> LateLintPass<'tcx> for NonThreadSafeCallInTest {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        if !cx.sess().opts.test {
            cx.sess().dcx().warn(
                "`non_thread_safe_call_in_test` is unlikely to be effective as `--test` was not \
                 passed to rustc",
            );
        }

        self.find_test_fns(cx);
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // smoelius: Don't emit warnings if there are less than two tests, since at least two
        // threads are needed for a race.
        if self.test_fns.len() >= 2 && self.is_test_item(item) {
            Checker {
                lint: self,
                cx,
                item,
                body_owner: item.owner_id.def_id,
            }
            .visit_item(item);
        }
    }
}

impl NonThreadSafeCallInTest {
    fn find_test_fns(&mut self, cx: &LateContext<'_>) {
        for item_id in cx.tcx.hir_free_items() {
            let item = cx.tcx.hir_item(item_id);
            // smoelius:
            // https://rustc-dev-guide.rust-lang.org/test-implementation.html#step-3-test-object-generation
            if let ItemKind::Const(_ident, _generics, ty, ConstItemRhs::Body(const_body_id)) = item.kind
                && let Some(ty_def_id) = ty.basic_res().opt_def_id()
                && match_def_path(cx, ty_def_id, &paths::TEST_DESC_AND_FN)
                && let const_body = cx.tcx.hir_body(const_body_id)
                && let ExprKind::Struct(_, fields, _) = const_body.value.kind
                && let Some(testfn) = fields.iter().find(|field| field.ident.as_str() == "testfn")
                // smoelius: Callee is `self::test::StaticTestFn`.
                && let ExprKind::Call(_, [arg]) = testfn.expr.kind
                && let ExprKind::Closure(Closure {
                    body: closure_body_id,
                    ..
                }) = arg.kind
                && let closure_body = cx.tcx.hir_body(*closure_body_id)
                // smoelius: Callee is `self::test::assert_test_result`.
                && let ExprKind::Call(_, [arg]) = closure_body.value.kind
                // smoelius: Callee is test function.
                && let ExprKind::Call(callee, _) = arg.kind
                && let Some(callee_def_id) = callee.basic_res().opt_def_id()
            {
                self.test_fns.push(callee_def_id);
            }
        }
    }

    fn is_test_item(&self, item: &Item) -> bool {
        self.test_fns
            .iter()
            .any(|&def_id| item.owner_id.to_def_id() == def_id)
    }
}

pub struct Checker<'cx, 'tcx> {
    lint: &'cx mut NonThreadSafeCallInTest,
    cx: &'cx LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    body_owner: LocalDefId,
}

impl<'tcx> Visitor<'tcx> for Checker<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, args) = expr.kind {
            if self.lint.visited_calls.contains(&expr.hir_id) {
                return;
            }

            let _ = self.lint.visited_calls.insert(expr.hir_id);

            let typeck = self.cx.tcx.typeck(self.body_owner);

            let Some(callee_def_id) = callee.res(typeck).opt_def_id() else {
                return;
            };

            if let Some(path) = is_blacklisted_function(self.cx, callee_def_id, callee, args) {
                span_lint_and_note(
                    self.cx,
                    NON_THREAD_SAFE_CALL_IN_TEST,
                    expr.span,
                    format!(
                        "calling `{}` in a test could affect the outcome of other tests",
                        path.join("::")
                    ),
                    self.item.kind.ident().map(|ident| ident.span),
                    "the call is reachable from at least this test",
                );
                return;
            }

            if let Some(local_def_id) = callee_def_id.as_local()
                && let Some(body) = self.cx.tcx.hir_maybe_body_owned_by(local_def_id)
            {
                let prev_body_owner = self.body_owner;
                self.body_owner = local_def_id;
                walk_body(self, body);
                self.body_owner = prev_body_owner;
                return;
            }
        }
        walk_expr(self, expr);
    }
}

fn is_blacklisted_function<'tcx>(
    cx: &LateContext<'tcx>,
    callee_def_id: DefId,
    callee: &Expr<'tcx>,
    args: &[Expr<'tcx>],
) -> Option<&'static [&'static str]> {
    let path = crate::blacklist::BLACKLIST
        .iter()
        .copied()
        .find(|path| match_def_path(cx, callee_def_id, path));

    // smoelius: Hack, until we can come up with a more general solution.
    if path == Some(&paths::COMMAND_NEW) && !command_new_additional_checks(cx, callee, args) {
        return None;
    }

    path
}

#[cfg_attr(dylint_lib = "supplementary", expect(commented_out_code))]
fn command_new_additional_checks(cx: &LateContext<'_>, callee: &Expr, args: &[Expr]) -> bool {
    if let [arg] = args
        && let ExprKind::Lit(lit) = arg.kind
        && let LitKind::Str(symbol, _) = lit.node
        && symbol.as_str() == "cargo"
    {
        for (_, node) in cx.tcx.hir_parent_iter(callee.hir_id) {
            if let Node::Expr(expr) = node
                && let ExprKind::MethodCall(method, _, args, _) = expr.kind
                // smoelius: We cannot call `LateContext::typeck_results` here because we might be
                // outside of a function body.
                // && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
                && (method.ident.as_str() == "arg" || method.ident.as_str() == "args")
                && let [arg] = args
            {
                if method.ident.as_str() == "arg" {
                    return is_lit_str_run(arg);
                } else if let ExprKind::Array(elts) = arg.kind
                    && let [elt, ..] = elts
                {
                    return is_lit_str_run(elt);
                }
                return false;
            }
        }
    }

    false
}

fn is_lit_str_run(expr: &Expr) -> bool {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Str(symbol, _) = lit.node
        && symbol.as_str() == "run"
    {
        true
    } else {
        false
    }
}
