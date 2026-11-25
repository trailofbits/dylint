#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_then, is_test_function};
use daggy::{Dag, NodeIndex};
use rustc_hir::{
    BodyId, Expr, ExprKind, ImplItemKind, Item, ItemKind, Mod,
    def_id::{DefId, LocalDefId, LocalModId},
    intravisit::{self, Visitor},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;
use serde::Deserialize;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// It enforces a relative order among module-level functions and associated functions and
    /// methods in `impl` blocks. Callers must precede their module-local callees. Functions called
    /// by the same caller should appear in call order. A constraint is rejected if it would create
    /// a cycle among functions or movable items, where each `impl` block moves as one item.
    ///
    /// ### Why is this bad?
    ///
    /// Without a certain order, it can be difficult to navigate through the module's functions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar() {}
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
    /// fn bar() {}
    /// ```
    ///
    /// ### Configuration
    ///
    /// - `check_test_functions: bool` (default `false`): Include functions marked with `#[test]` in
    ///   the ordering checks.
    ///
    /// ### Known problems
    ///
    /// - While the lint may seem strict, its rules do not completely dictate the order of a
    ///   module's functions. Judgement must often be exercised to address the lint's warnings.
    /// - Calls through trait methods may not constrain the position of their concrete
    ///   implementation.
    pub NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
    Warn,
    "Enforce callers before callees and compatible call ordering among module-local functions",
    NonTopologicallySortedFunctions::new()
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Config {
    check_test_functions: bool,
}

struct NonTopologicallySortedFunctions {
    config: Config,
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

struct Finder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    seen: HashSet<LocalDefId>,
    #[rustfmt::skip] // https://github.com/rust-lang/rustfmt/issues/6180
    /// Distinct callees in order of first occurrence during a preorder traversal of the body.
    ///
    /// Each element stores:
    ///
    /// - The `LocalDefId` of the callee
    /// - The `Span` of the call site
    ///
    /// This order determines proposed constraints between callees. For example, calls to `bar()`
    /// followed by `baz()` propose that `bar` be defined before `baz`. A cycle may cause the
    /// constraint to be rejected.
    ///
    /// The `Span` is later used to produce more precise diagnostics; if a function is out of order,
    /// we can point to the exact call site that implies the constraint.
    order: Vec<Callee>,
}

impl<'tcx> Visitor<'tcx> for Finder<'_, 'tcx> {
    type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
        let callee_def_id = match ex.kind {
            ExprKind::Call(callee, _) => {
                if let ExprKind::Path(ref qpath) = callee.kind {
                    self.cx.qpath_res(qpath, callee.hir_id).opt_def_id()
                } else {
                    None
                }
            }
            ExprKind::MethodCall(..) => self
                .cx
                .tcx
                .typeck(ex.hir_id.owner.def_id)
                .type_dependent_def_id(ex.hir_id),
            _ => None,
        };

        if let Some(local_def_id) = callee_def_id.and_then(DefId::as_local)
            && self.seen.insert(local_def_id)
        {
            self.order.push(Callee {
                callee_local_def_id: local_def_id,
                call_span: ex.span,
            });
        }

        intravisit::walk_expr(self, ex);
    }
}

impl<'tcx> LateLintPass<'tcx> for NonTopologicallySortedFunctions {
    fn check_mod(
        &mut self,
        cx: &LateContext<'tcx>,
        module: &'tcx Mod<'tcx>,
        _module_id: LocalModId,
    ) {
        // Collect module-level functions and associated functions and methods from `impl` blocks
        // in source order.
        let mut def_order: Vec<LocalDefId> = vec![];
        let mut functions: HashMap<LocalDefId, Span> = HashMap::new();
        let mut unit_spans: HashMap<LocalDefId, Span> = HashMap::new();
        let mut function_unit_map: HashMap<LocalDefId, LocalDefId> = HashMap::new();
        let mut add_function = |local_def_id, span, unit, unit_span| {
            if self.config.check_test_functions || !is_test_function(cx.tcx, local_def_id) {
                def_order.push(local_def_id);
                functions.insert(local_def_id, span);
                function_unit_map.insert(local_def_id, unit);
                unit_spans.insert(unit, unit_span);
            }
        };

        for item_id in module.item_ids {
            let item: &Item<'tcx> = cx.tcx.hir_item(*item_id);
            match item.kind {
                ItemKind::Fn { .. } => add_function(
                    item.owner_id.def_id,
                    item.span,
                    item.owner_id.def_id,
                    item.span,
                ),
                ItemKind::Impl(impl_) => {
                    for impl_item_id in impl_.items {
                        let impl_item = cx.tcx.hir_impl_item(*impl_item_id);
                        if let ImplItemKind::Fn(..) = impl_item.kind {
                            add_function(
                                impl_item.owner_id.def_id,
                                impl_item.span,
                                item.owner_id.def_id,
                                item.span,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        if def_order.len() < 2 {
            return;
        }

        let mut constraints = ConstraintGraph::new(function_unit_map);
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

        let violations = Self::find_violations(cx, &constraints, &functions, &unit_spans);

        for violation in violations {
            Self::report_violation(cx, violation);
        }
    }
}

impl NonTopologicallySortedFunctions {
    fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }

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
            // earlier callers (callers are visited in source order).
            let _: bool = constraints.add(
                caller_id,
                callee_local_def_id,
                ConstraintReason::CallerCallee { call_span },
            );
        }
    }

    /// Build call-order constraints: if a caller calls `foo` before `bar`, then `foo` must be
    /// defined before `bar`.
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
        unit_spans: &HashMap<LocalDefId, Span>,
    ) -> Vec<Violation> {
        let mut violations: Vec<Violation> = constraints
            .function_dag
            .raw_edges()
            .iter()
            .filter_map(|edge| {
                let a = constraints.function_dag[edge.source()];
                let b = constraints.function_dag[edge.target()];
                let unit_a = constraints.unit_of(a);
                let unit_b = constraints.unit_of(b);
                let within_unit = unit_a == unit_b;
                // Compare whole items across units, but individual functions within one `impl`.
                let span_a = if within_unit {
                    functions.get(&a)?
                } else {
                    unit_spans.get(&unit_a)?
                };
                let span_b = if within_unit {
                    functions.get(&b)?
                } else {
                    unit_spans.get(&unit_b)?
                };
                // Expanded definitions can have spans in the macro's source file. Compare their
                // invocation locations, while retaining the original spans for diagnostics.
                if span_a.source_callsite().lo() > span_b.source_callsite().hi() {
                    let name_a = cx.tcx.def_path_str(a.to_def_id());
                    let name_b = cx.tcx.def_path_str(b.to_def_id());
                    let id_second_item = if within_unit { b } else { unit_b };
                    let move_kind = if unit_b == b {
                        MoveKind::FunctionInModule
                    } else if within_unit {
                        MoveKind::FunctionWithinImpl
                    } else {
                        MoveKind::ImplBlock
                    };
                    let violation = Violation {
                        span_first_item: *span_a,
                        span_second_item: *span_b,
                        id_second_item,
                        path_second_item: cx.tcx.def_path_str(id_second_item.to_def_id()),
                        name_first_fn: name_a,
                        name_second_fn: name_b,
                        move_kind,
                        reason: edge.weight,
                    };
                    Some(violation)
                } else {
                    None
                }
            })
            .collect();

        // Sort by the item that must move, then by its required predecessors in
        // reverse source order.
        violations.sort_by(Self::compare_violations);

        // Multiple constraints can target one method or `impl` block. Keep the constraint
        // whose required predecessor appears latest in source order.
        violations.dedup_by_key(|violation| violation.id_second_item);

        violations
    }

    fn compare_violations(a: &Violation, b: &Violation) -> Ordering {
        a.span_second_item
            .source_callsite()
            .lo()
            .cmp(&b.span_second_item.source_callsite().lo())
            // Multiple items can share an invocation span, and multiple functions can belong to
            // one target `impl`. Group by the item that must move before comparing predecessors.
            .then(a.path_second_item.cmp(&b.path_second_item))
            // For each target item, sort required predecessors in reverse source order.
            .then(
                b.span_first_item
                    .source_callsite()
                    .lo()
                    .cmp(&a.span_first_item.source_callsite().lo()),
            )
            .then(a.name_first_fn.cmp(&b.name_first_fn))
    }

    fn report_violation(cx: &LateContext<'_>, violation: Violation) {
        let Violation {
            name_first_fn,
            name_second_fn,
            span_second_item,
            move_kind,
            reason,
            ..
        } = violation;
        span_lint_and_then(
            cx,
            NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
            span_second_item,
            "function definitions are not topologically sorted",
            |diag| {
                diag.span_label(
                    span_second_item,
                    format!(
                        "function `{name_second_fn}` should be defined after `{name_first_fn}`"
                    ),
                );

                diag.help(match move_kind {
                    MoveKind::FunctionInModule => {
                        format!("move {name_second_fn}'s definition to later in the module")
                    }
                    MoveKind::FunctionWithinImpl => {
                        format!("move {name_second_fn}'s definition to later in the impl block")
                    }
                    MoveKind::ImplBlock => format!(
                        "move the impl block containing `{name_second_fn}` to later in the module"
                    ),
                });

                match reason {
                    ConstraintReason::CallerCallee { call_span } => {
                        diag.span_note(
                            call_span,
                            format!("`{name_second_fn}` is called from `{name_first_fn}` here"),
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

#[derive(Default)]
struct ConstraintGraph {
    /// Stores function `LocalDefId`s as nodes and accepted ordering constraints as edges whose
    /// weights explain why the constraint exists.
    function_dag: Dag<LocalDefId, ConstraintReason>,
    /// Maps each function's `LocalDefId` to the `NodeIndex` required by `daggy`, ensuring that all
    /// constraints involving the same function use the same node.
    function_node_indices: HashMap<LocalDefId, NodeIndex>,
    /// An `impl` block moves as one unit, while each free function moves independently.
    unit_dag: Dag<LocalDefId, ()>,
    unit_node_indices: HashMap<LocalDefId, NodeIndex>,
    function_unit_map: HashMap<LocalDefId, LocalDefId>,
}

impl ConstraintGraph {
    fn new(function_unit_map: HashMap<LocalDefId, LocalDefId>) -> Self {
        Self {
            function_unit_map,
            ..Self::default()
        }
    }

    fn add(&mut self, before: LocalDefId, after: LocalDefId, reason: ConstraintReason) -> bool {
        let before_index = *self
            .function_node_indices
            .entry(before)
            .or_insert_with(|| self.function_dag.add_node(before));
        let after_index = *self
            .function_node_indices
            .entry(after)
            .or_insert_with(|| self.function_dag.add_node(after));

        if self
            .function_dag
            .find_edge(before_index, after_index)
            .is_some()
        {
            return true;
        }

        let before_unit = self.unit_of(before);
        let after_unit = self.unit_of(after);

        if before_unit != after_unit {
            let before_unit_index = *self
                .unit_node_indices
                .entry(before_unit)
                .or_insert_with(|| self.unit_dag.add_node(before_unit));
            let after_unit_index = *self
                .unit_node_indices
                .entry(after_unit)
                .or_insert_with(|| self.unit_dag.add_node(after_unit));

            // A function-level DAG alone can accept `method_a < free_fn < method_b`, even
            // though both methods belong to the same indivisible `impl` block.
            if self
                .unit_dag
                .find_edge(before_unit_index, after_unit_index)
                .is_none()
                && self
                    .unit_dag
                    .add_edge(before_unit_index, after_unit_index, ())
                    .is_err()
            {
                return false;
            }
        }

        // Cross-unit cycles were ruled out above, but adding a function edge could still
        // create a cycle within one unit.
        let added = self
            .function_dag
            .add_edge(before_index, after_index, reason)
            .is_ok();
        debug_assert!(before_unit == after_unit || added);
        added
    }

    fn unit_of(&self, function: LocalDefId) -> LocalDefId {
        // Callees outside the current module are not collected as movable units. Keep treating
        // them as individual nodes, as the function graph already does.
        self.function_unit_map
            .get(&function)
            .copied()
            .unwrap_or(function)
    }
}

enum MoveKind {
    FunctionInModule,
    FunctionWithinImpl,
    ImplBlock,
}

struct Violation {
    name_first_fn: String,
    name_second_fn: String,
    id_second_item: LocalDefId,
    path_second_item: String,
    span_first_item: Span,
    span_second_item: Span,
    move_kind: MoveKind,
    reason: ConstraintReason,
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

// smoelius: False negative.
#[test]
fn ui_attr() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui_attr");
}

#[test]
fn ui_ignore_test_functions() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_ignore_test_functions")
        .rustc_flags(["--test"])
        .run();
}

#[test]
fn ui_check_test_functions() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_check_test_functions")
        .rustc_flags(["--test"])
        .dylint_toml("non_topologically_sorted_functions.check_test_functions = true")
        .run();
}

#[test]
fn ui_main_rs_are_equal() {
    let ui_ignore_test_functions_main_rs =
        std::fs::read_to_string("ui_ignore_test_functions/main.rs").unwrap();

    let ui_check_test_functions_main_rs =
        std::fs::read_to_string("ui_check_test_functions/main.rs").unwrap();

    assert_eq!(
        ui_ignore_test_functions_main_rs,
        ui_check_test_functions_main_rs
    );
}
