#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_then;
use daggy::{Dag, NodeIndex};
use rustc_hir::def::Res;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{BodyId, Expr, ExprKind, HirId, Item, ItemKind, Mod};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;
use std::collections::{HashMap, HashSet};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// It enforces a relative order among functions defined within a module. Callers must precede
    /// their module-local callees. Functions called by the same caller should appear in call order,
    /// unless that ordering would conflict with caller-callee constraints. Caller-callee
    /// constraints take precedence, and a call-order constraint is ignored if adding it would
    /// create a cycle.
    ///
    /// ### Why is this bad?
    ///
    /// Without a certain order, it can be difficult to navigate through the module's functions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar() { }
    ///
    /// fn foo() {
    ///     bar();
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn foo() {
    ///     bar();
    /// }
    ///
    /// fn bar() { }
    /// ```
    pub NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
    Warn,
    "Enforce callers before callees and compatible call ordering among module-local functions"
}

struct Callee {
    pub callee_local_def_id: LocalDefId,
    pub call_span: Span,
}

/// Explains why function `foo` must come before function `bar`.
#[derive(Clone, Copy)]
enum ConstraintReason {
    /// `foo` calls `bar` at the given span.
    CallerCallee { call_span: Span },
    /// `foo` and `bar` are both called by `caller`, with `foo` called first.
    CallOrder {
        caller: LocalDefId,
        first_call_span: Span,
        second_call_span: Span,
    },
}

#[derive(Default)]
struct ConstraintGraph {
    /// Stores function `LocalDefId`s as nodes and accepted ordering constraints as edges whose
    /// weights explain why the constraint exists.
    dag: Dag<LocalDefId, ConstraintReason>,
    /// Maps each function's `LocalDefId` to the `NodeIndex` required by `daggy`, ensuring that all
    /// constraints involving the same function use the same node.
    node_indices: HashMap<LocalDefId, NodeIndex>,
}

impl ConstraintGraph {
    fn add(&mut self, before: LocalDefId, after: LocalDefId, reason: ConstraintReason) -> bool {
        let before_index = *self
            .node_indices
            .entry(before)
            .or_insert_with(|| self.dag.add_node(before));
        let after_index = *self
            .node_indices
            .entry(after)
            .or_insert_with(|| self.dag.add_node(after));

        self.dag.find_edge(before_index, after_index).is_some()
            || self.dag.add_edge(before_index, after_index, reason).is_ok()
    }
}

struct Finder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    seen: HashSet<LocalDefId>,
    /// The list of callees encountered during a preorder traversal of the body.
    ///
    /// Each element stores:
    ///
    /// - The `LocalDefId` of the callee
    /// - The `Span` of the call site
    ///
    /// This ordering is significant: the first occurrence of a callee defines
    /// how constraints between callees are derived. For example, if calls appear
    /// in the order `bar()`, then `baz()`, this produces the ordering constraint
    /// `bar` must come before `baz` when functions are arranged in the module.
    ///
    /// The `Span` is later used to produce more precise diagnostics; if a
    /// function is out of order, we can point to the exact call site that
    /// implies the constraint.
    order: Vec<Callee>,
}

impl<'tcx> Visitor<'tcx> for Finder<'_, 'tcx> {
    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _args) = &ex.kind
            && let ExprKind::Path(ref qpath) = callee.kind
            && let res = self.cx.qpath_res(qpath, callee.hir_id)
            && let Res::Def(_, def_id) = res
            && let Some(local_def_id) = def_id.as_local()
            && !self.seen.contains(&local_def_id)
        {
            self.seen.insert(local_def_id);
            self.order.push(Callee {
                callee_local_def_id: local_def_id,
                call_span: ex.span,
            });
        }

        // keep traversing
        intravisit::walk_expr(self, ex);
    }
}

impl NonTopologicallySortedFunctions {
    fn collect_callees_in_body(cx: &LateContext<'_>, body_id: BodyId) -> Vec<Callee> {
        let body = cx.tcx.hir_body(body_id);
        let mut finder = Finder {
            cx,
            seen: HashSet::new(),
            order: Vec::new(),
        };
        intravisit::walk_body(&mut finder, body);
        finder.order
    }

    /// Build caller-callee constraints: each caller must come before its callees.
    fn build_caller_callee_constraint(
        caller_id: LocalDefId,
        callees: &[Callee],
        constraints: &mut ConstraintGraph,
    ) {
        for &Callee {
            callee_local_def_id,
            call_span,
        } in callees
        {
            // If this constraint would introduce a cycle, keep the constraints added by
            // earlier callers (callers are visited in module order).
            let _: bool = constraints.add(
                caller_id,
                callee_local_def_id,
                ConstraintReason::CallerCallee { call_span },
            );
        }
    }

    /// Build call-order constraints: if a caller calls `foo` before `bar`, then `foo`
    /// must come before `bar` in the module.
    fn build_call_order_constraints(
        caller_id: LocalDefId,
        callees: &[Callee],
        constraints: &mut ConstraintGraph,
    ) {
        for i in 0..callees.len() {
            for j in (i + 1)..callees.len() {
                let a = callees[i].callee_local_def_id;
                let b = callees[j].callee_local_def_id;
                // Caller-callee constraints take precedence over call-order constraints.
                // Skip this call-order constraint if adding it would close a cycle in the
                // constraints accumulated thus far.
                let _: bool = constraints.add(
                    a,
                    b,
                    ConstraintReason::CallOrder {
                        caller: caller_id,
                        first_call_span: callees[i].call_span,
                        second_call_span: callees[j].call_span,
                    },
                );
            }
        }
    }

    fn find_violations(
        cx: &LateContext<'_>,
        constraints: &ConstraintGraph,
        functions: &HashMap<LocalDefId, Span>,
    ) -> Vec<Violation> {
        let mut violations: Vec<Violation> = constraints
            .dag
            .raw_edges()
            .iter()
            .filter_map(|edge| {
                let a = constraints.dag[edge.source()];
                let b = constraints.dag[edge.target()];
                let span_a = functions.get(&a)?;
                let span_b = functions.get(&b)?;
                if span_a.lo() > span_b.hi() {
                    let span = *span_a;
                    let name_a = cx.tcx.def_path_str(a.to_def_id());
                    let name_b = cx.tcx.def_path_str(b.to_def_id());
                    let violation = Violation {
                        span,
                        id_first_fn: a,
                        name_first_fn: name_a,
                        name_second_fn: name_b,
                        reason: edge.weight,
                    };
                    Some(violation)
                } else {
                    None
                }
            })
            .collect();

        // keep the same order: sort deterministically by span.lo, span.hi, name
        violations.sort_by(
            |Violation {
                 name_first_fn: name_a,
                 span: span_a,
                 ..
             },
             Violation {
                 name_first_fn: name_b,
                 span: span_b,
                 ..
             }| {
                span_a
                    .lo()
                    .cmp(&span_b.lo())
                    .then(span_a.hi().cmp(&span_b.hi()))
                    .then(name_a.as_str().cmp(name_b.as_str()))
            },
        );

        violations
    }
}

struct Violation {
    name_first_fn: String,
    name_second_fn: String,
    id_first_fn: LocalDefId,
    span: Span,
    reason: ConstraintReason,
}

impl<'tcx> LateLintPass<'tcx> for NonTopologicallySortedFunctions {
    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _module_id: HirId) {
        // Collect top-level functions
        let mut def_order: Vec<LocalDefId> = vec![];
        let mut functions: HashMap<LocalDefId, Span> = HashMap::new();

        for item_id in module.item_ids {
            let item: &Item<'tcx> = cx.tcx.hir_item(*item_id);
            if let ItemKind::Fn { .. } = item.kind {
                let local_def_id = item.owner_id.def_id;

                def_order.push(local_def_id);
                functions.insert(local_def_id, item.span);
            }
        }

        if def_order.len() < 2 {
            return;
        }

        let mut constraints = ConstraintGraph::default();
        let mut calls = Vec::new();

        for caller_id in def_order {
            let caller_body = cx.tcx.hir_maybe_body_owned_by(caller_id);

            if let Some(caller_body) = caller_body {
                let caller_body_id = caller_body.id();
                let callees: Vec<Callee> = Self::collect_callees_in_body(cx, caller_body_id);

                Self::build_caller_callee_constraint(caller_id, &callees, &mut constraints);

                calls.push((caller_id, callees));
            }
        }

        for (caller_id, callees) in calls {
            Self::build_call_order_constraints(caller_id, &callees, &mut constraints);
        }

        let violations = Self::find_violations(cx, &constraints, &functions);

        // A function can violate multiple outgoing constraints, but emitting all of them would
        // place multiple warnings on the same definition. Because violations are sorted
        // deterministically above, report the first violation for each misplaced function and
        // suppress the rest.
        let mut warned: HashSet<LocalDefId> = HashSet::new();

        for violation in violations {
            let Violation {
                name_first_fn,
                name_second_fn,
                id_first_fn,
                span,
                reason,
            } = violation;
            if warned.insert(id_first_fn) {
                span_lint_and_then(
                    cx,
                    NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
                    span,
                    "function definitions are not topologically sorted",
                    |diag| {
                        diag.span_label(
                            span,
                            format!(
                                "function `{name_first_fn}` should be defined before `{name_second_fn}`"
                            ),
                        );

                        diag.help(format!(
                            "move {name_first_fn}'s definition to earlier in the module"
                        ));

                        match reason {
                            ConstraintReason::CallerCallee { call_span } => {
                                diag.span_note(
                                    call_span,
                                    format!(
                                        "`{name_second_fn}` is called from `{name_first_fn}` here"
                                    ),
                                );
                            }
                            ConstraintReason::CallOrder {
                                caller,
                                first_call_span,
                                second_call_span,
                            } => {
                                let caller_name = cx.tcx.def_path_str(caller.to_def_id());
                                diag.span_note(
                                    first_call_span,
                                    format!("`{caller_name}` calls `{name_first_fn}` here"),
                                );
                                diag.span_note(
                                    second_call_span,
                                    format!("`{caller_name}` calls `{name_second_fn}` here"),
                                );
                            }
                        }
                    },
                );
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
