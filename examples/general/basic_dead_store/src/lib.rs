#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_help, get_parent_expr};
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Finds instances of dead stores in arrays: array positions that are assigned twice without a
    /// use or read in between.
    ///
    /// ### Why is this bad?
    ///
    /// A dead store might indicate a logic error in the program or an unnecessary assignment.
    ///
    /// ### Known problems
    ///
    /// This lint only checks for literal indices and will not try to find instances where an array
    /// is indexed by a variable.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let mut arr = [0u64; 2];
    /// arr[0] = 1;
    /// arr[0] = 2;
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let mut arr = [0u64; 2];
    /// arr[0] = 2;
    /// arr[1] = 1;
    /// ```
    pub BASIC_DEAD_STORE,
    Warn,
    "An array element is assigned twice without a use or read in between",
    BasicDeadStore::default()
}

#[derive(Default)]
pub struct BasicDeadStore {
    /// Stores instances of array-indexing with literal (array name, index, span)
    arr_and_idx_vec: Vec<(String, u128, Span)>,
}

impl BasicDeadStore {
    /// Removes all stored values of the given array
    fn clear_stores_of(&mut self, string: &str) {
        self.arr_and_idx_vec
            .retain(|(arr_string, _idx, _span)| arr_string != string);
    }

    /// Returns all instances where the given array is indexed at `idx`
    fn get_pairs_with_same_name_idx(
        &self,
        string: &String,
        idx: &u128,
    ) -> Vec<&(String, u128, Span)> {
        self.arr_and_idx_vec
            .iter()
            .filter(|(arr_string, arr_idx, _span)| arr_string == string && arr_idx == idx)
            .collect()
    }
}

/// Checks if the given expression is an assignment to an array indexed by a literal.
/// This consists in starting with `expr` and checking that the other nodes in the tree are of the
/// correct kind:
///
/// ```ignore
///                 assign_expr is ExprKind::Assign
///                      |
///                   index_expr is ExprKind::Index
///                  /         \
///      |start|-> expr        lit is ExprKind::Lit && LitKind::Int
/// ```
///
/// Returns the tuple (indexed position, span)
fn is_assignment_to_array_indexed_by_literal(
    expr: &Expr,
    tcx: &LateContext<'_>,
) -> Option<(u128, Span)> {
    let index_expr = get_parent_expr(tcx, expr)?;
    if let ExprKind::Index(array, index, _span) = index_expr.kind
        && array.hir_id == expr.hir_id
        && let assign_expr = get_parent_expr(tcx, index_expr)?
        && let ExprKind::Assign(target, _value, assignment_span) = assign_expr.kind
        && target.hir_id == index_expr.hir_id
        && let ExprKind::Lit(lit) = index.kind
        && let LitKind::Int(index, _type) = lit.node
    {
        return Some((index.get(), assignment_span));
    }
    None
}

impl<'tcx> LateLintPass<'tcx> for BasicDeadStore {
    // Given an Expression:
    //  - If we are looking at an array being indexed by a literal:
    //      - if we have seen this array being indexed at this literal before
    //          - then we found a dead store
    //      - otherwise, we save this instance in a vec V
    //  - Otherwise, clear all stored instances of this expression in the vec V
    fn check_expr(&mut self, ctx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Path(ref qpath) = expr.kind {
            let array_resolution = ctx.qpath_res(qpath, expr.hir_id);
            let array_name = format!("{array_resolution:?}");

            if let Some((v, span)) = is_assignment_to_array_indexed_by_literal(expr, ctx) {
                let in_common = self.get_pairs_with_same_name_idx(&array_name, &v);
                if in_common.is_empty() {
                    // If there are no saved instances of this array being assigned indexed at the
                    // same index, save this instance
                    self.arr_and_idx_vec.push((array_name, v, span));
                } else {
                    // Otherwise, we are storing again into the same index
                    // unwrap: `in_common` is guaranteed to have at least one element
                    span_lint_and_help(
                        ctx,
                        BASIC_DEAD_STORE,
                        span,
                        "reassigning the same array position without using it",
                        Some(in_common.first().unwrap().2),
                        "original assignment was here",
                    );
                }
            } else {
                // If we are using the array in a way that is not an assignment to a certain
                // position, then it is being used. Therefore, we need to clear all
                // stored instances of this array
                self.clear_stores_of(&array_name);
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
