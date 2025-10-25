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
    "Enforce callers before callees and consistent order of callees (module-local functions)"
}

struct Callee {
    pub caller_local_def_id: LocalDefId,
    pub call_span: Span,
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
                caller_local_def_id: local_def_id,
                call_span: ex.span,
            }); // capture call span
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

    /// Collect all funcs in caller's body and place them like (caller -> (callee, `call_span`))
    fn build_caller_callee_constraint(
        caller_id: LocalDefId,
        callees: &[Callee],
        mut must_come_before: HashSet<(LocalDefId, LocalDefId)>,
        call_sites: &mut HashMap<(LocalDefId, LocalDefId), Option<Span>>,
    ) -> HashSet<(LocalDefId, LocalDefId)> {
        for &Callee {
            caller_local_def_id,
            call_span,
        } in callees
        {
            let key = (caller_id, caller_local_def_id);
            // (caller -> callee) constraint
            // If the reverse constraint already exists (added by an earlier caller),
            // we keep the earlier constraint (because we iterate callers in module order).
            if must_come_before.contains(&(caller_local_def_id, caller_id)) {
                // reversed constraint exists; skip adding (precedence kept)
            } else {
                must_come_before.insert(key);
            }

            call_sites.entry(key).or_insert(Some(call_span));
        }
        must_come_before
    }

    /// Check inner order rule.
    ///
    /// The earlier order is preferred and is considered the main one.
    fn transitive_closure(
        callees: &[Callee],
        mut must_come_before: HashSet<(LocalDefId, LocalDefId)>,
    ) -> HashSet<(LocalDefId, LocalDefId)> {
        // We only need the LocalDefId ordering here (ignore spans)
        let ids: Vec<LocalDefId> = callees
            .iter()
            .map(
                |&Callee {
                     caller_local_def_id,
                     ..
                 }| caller_local_def_id,
            )
            .collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = ids[i];
                let b = ids[j];
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
        functions: &HashMap<LocalDefId, Span>,
    ) -> Vec<Violation> {
        let mut violations: Vec<Violation> = must_come_before
            .iter()
            .filter_map(|&(a, b)| {
                let span_a = functions.get(&a)?;
                let span_b = functions.get(&b)?;
                if span_a.lo() > span_b.hi() {
                    let span = *span_a;
                    let name_a = cx.tcx.def_path_str(a.to_def_id());
                    let name_b = cx.tcx.def_path_str(b.to_def_id());
                    let violation = Violation {
                        span,
                        id_first_fn: a,
                        id_second_fn: b,
                        name_first_fn: name_a,
                        name_second_fn: name_b,
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

#[derive(Debug, Clone)]
struct Violation {
    name_first_fn: String,
    name_second_fn: String,
    id_first_fn: LocalDefId,
    id_second_fn: LocalDefId,
    span: Span,
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

        let mut must_come_before: HashSet<(LocalDefId, LocalDefId)> = HashSet::new();
        // stores all call sites for (caller, callee)
        let mut call_sites: HashMap<(LocalDefId, LocalDefId), Option<Span>> = HashMap::new();

        for caller_id in def_order {
            // use hir_maybe_body_owned_by — works for functions and methods etc.
            let caller_body = cx.tcx.hir_maybe_body_owned_by(caller_id);

            if let Some(caller_body) = caller_body {
                let caller_body_id = caller_body.id();
                let callees: Vec<Callee> = Self::collect_callees_in_body(cx, caller_body_id);

                must_come_before = Self::build_caller_callee_constraint(
                    caller_id,
                    &callees,
                    must_come_before,
                    &mut call_sites,
                );
                must_come_before = Self::transitive_closure(&callees, must_come_before);
            }
        }

        let violations = Self::find_violations(cx, &must_come_before, &functions);
        let mut warned: HashSet<LocalDefId> = HashSet::new();

        for violation in violations {
            let Violation {
                name_first_fn,
                name_second_fn,
                id_first_fn,
                id_second_fn,
                span,
                ..
            } = violation;
            if warned.insert(id_first_fn) {
                cx.span_lint(NON_TOPOLOGICALLY_SORTED_FUNCTIONS, span, |diag| {
                    // PRIMARY ERROR
                    diag.span_label(
                        span,
                        format!(
                            "function `{name_first_fn}` should be defined before `{name_second_fn}`"
                        ),
                    );

                    diag.help(format!(
                        "move {name_first_fn}'s definition to earlier in the module"
                    ));

                    // search call sites
                    if let Some(sites) = call_sites.get(&(id_first_fn, id_second_fn))
                        && let Some(call_span) = sites
                    {
                        // extra block with double info
                        diag.span_note(
                            *call_span,
                            format!("`{name_second_fn}` is called from `{name_first_fn}` here"),
                        );
                    }
                });
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
