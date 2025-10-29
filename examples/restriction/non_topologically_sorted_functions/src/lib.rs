#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use rustc_hir::def::Res;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{BodyId, Expr, ExprKind, HirId, Item, ItemKind, Mod};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use std::collections::{HashMap, HashSet};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// It enforces a certain relative order among functions defined within a module.
    ///
    /// ### Why is this bad?
    ///
    /// Without a certain order it's really bad to navigate through the modules.
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
    "Enforce callers before callees and consistent order of callees (module-local functions)"
}

struct Finder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    local_defs: &'a HashMap<LocalDefId, Span>,
    seen: HashSet<LocalDefId>,
    order: Vec<LocalDefId>,
}

impl<'tcx> Visitor<'tcx> for Finder<'_, 'tcx> {
    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _args) = &ex.kind
            && let ExprKind::Path(ref qpath) = callee.kind
        {
            let res = self.cx.qpath_res(qpath, callee.hir_id);
            if let Res::Def(_, def_id) = res
                && let Some(local_def_id) = def_id.as_local()
                && self.local_defs.contains_key(&local_def_id)
                && !self.seen.contains(&local_def_id)
            {
                self.seen.insert(local_def_id);
                self.order.push(local_def_id);
            }
        }

        // keep traversing
        intravisit::walk_expr(self, ex);
    }
}

impl<'tcx> NonTopologicallySortedFunctions {
    fn find_caller_body(
        cx: &LateContext<'tcx>,
        module: &'tcx Mod<'tcx>,
        caller_id: LocalDefId,
    ) -> Option<BodyId> {
        let mut caller_body: Option<BodyId> = None;

        for item_id in module.item_ids {
            let item = cx.tcx.hir_item(*item_id);
            if let ItemKind::Fn { body, .. } = item.kind
                && item.owner_id.def_id == caller_id
            {
                caller_body = Some(body);
                break;
            }
        }

        caller_body
    }

    fn collect_callees_in_body(
        cx: &LateContext<'_>,
        body_id: BodyId,
        local_defs: &HashMap<LocalDefId, Span>,
    ) -> Vec<LocalDefId> {
        let body = cx.tcx.hir_body(body_id);
        let mut finder = Finder {
            cx,
            local_defs,
            seen: HashSet::new(),
            order: Vec::new(),
        };
        intravisit::walk_body(&mut finder, body);
        finder.order
    }

    /// Collect all funcs in caller's body and place them like (caller -> callee)
    fn build_caller_callee_constraint(
        caller_id: LocalDefId,
        callees: &[LocalDefId],
        mut must_come_before: HashSet<(LocalDefId, LocalDefId)>,
    ) -> HashSet<(LocalDefId, LocalDefId)> {
        for &callee_id in callees {
            // (caller -> callee) constraint
            // If the reverse constraint already exists (added by an earlier caller),
            // we keep the earlier constraint (because we iterate callers in module order).
            if must_come_before.contains(&(callee_id, caller_id)) {
                // reversed constraint exists; skip adding (precedence kept)
            } else {
                must_come_before.insert((caller_id, callee_id));
            }
        }
        must_come_before
    }

    /// Check inner order rule.
    ///
    /// The earlier order is preferred and is considered the main one.
    fn build_multiple_precedence_rule(
        callees: &[LocalDefId],
        mut must_come_before: HashSet<(LocalDefId, LocalDefId)>,
    ) -> HashSet<(LocalDefId, LocalDefId)> {
        for i in 0..callees.len() {
            for j in (i + 1)..callees.len() {
                let a = callees[i];
                let b = callees[j];
                // prefer earlier constraint: if (b,a) already exists, skip
                if must_come_before.contains(&(b, a)) {
                    // earlier caller already set reversed ordering; keep it.
                    continue;
                }
                must_come_before.insert((a, b));
            }
        }
        must_come_before
    }

    fn find_violations(
        cx: &LateContext<'_>,
        must_come_before: &HashSet<(LocalDefId, LocalDefId)>,
        spans: &HashMap<LocalDefId, Span>,
    ) -> Vec<Violation> {
        let mut violations: Vec<Violation> = must_come_before
            .iter()
            .filter_map(|&(a, b)| {
                let span_a = spans.get(&a)?;
                let span_b = spans.get(&b)?;
                if span_a.lo() > span_b.hi() {
                    let span = spans
                        .get(&a)
                        .copied()
                        .expect("Has to be fn meta for function in module");
                    let name_a = cx.tcx.def_path_str(a.to_def_id());
                    let name_b = cx.tcx.def_path_str(b.to_def_id());
                    let violation = Violation {
                        span,
                        id_first_fn: a,
                        name_first_fn: name_a,
                        name_second_fn: name_b,
                    };
                    Some(violation)
                } else {
                    None
                }
            })
            .collect();

        // keep the same order
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
                // ia1.cmp(ia2)
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
}

impl<'tcx> LateLintPass<'tcx> for NonTopologicallySortedFunctions {
    // A list of things you might check can be found here:
    // https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html

    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _module_id: HirId) {
        // Collect top-level functions
        let mut def_order: Vec<LocalDefId> = vec![];
        let mut spans: HashMap<LocalDefId, Span> = HashMap::new();

        for item_id in module.item_ids {
            let item: &Item<'tcx> = cx.tcx.hir_item(*item_id);
            if let ItemKind::Fn { .. } = item.kind {
                let local_def_id = item.owner_id.def_id;

                def_order.push(local_def_id);
                spans.insert(local_def_id, item.span);
            }
        }

        if def_order.len() < 2 {
            return;
        }

        let mut must_come_before: HashSet<(LocalDefId, LocalDefId)> = HashSet::new();

        for caller_id in def_order {
            let caller_body = Self::find_caller_body(cx, module, caller_id);

            if let Some(caller_body_id) = caller_body {
                let callees: Vec<LocalDefId> =
                    Self::collect_callees_in_body(cx, caller_body_id, &spans);

                must_come_before =
                    Self::build_caller_callee_constraint(caller_id, &callees, must_come_before);
                must_come_before = Self::build_multiple_precedence_rule(&callees, must_come_before);
            }
        }

        let violations = Self::find_violations(cx, &must_come_before, &spans);
        let mut warned: HashSet<LocalDefId> = HashSet::new();

        for Violation {
            name_first_fn,
            name_second_fn,
            id_first_fn,
            span,
            ..
        } in violations
        {
            if warned.insert(id_first_fn) {
                cx.span_lint(NON_TOPOLOGICALLY_SORTED_FUNCTIONS, span, |diag| {
                    diag.span_label(span, format!("function `{name_first_fn}` should be defined before `{name_second_fn}`"));
                    diag.help("move the function earlier in the module so callers and callee ordering is respected");
                });
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
