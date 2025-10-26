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
    ///  ### What it does
    ///
    ///  It enforces a certain relative order among functions defined within a module.
    ///
    ///  ### Why is this bad?
    ///
    ///  Without a certain order it's really bad to navigate through the modules.
    ///
    ///  ### Example
    ///
    ///  ```rust
    ///  fn bar() { }
    ///
    ///  fn foo() {
    ///      bar();
    ///  }
    ///  ```
    ///
    ///  Use instead:
    ///
    ///  ```rust
    ///  fn foo() {
    ///      bar();
    ///  }
    ///
    ///  fn bar() { }
    ///  ```
    pub NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
    Warn,
    "Enforce callers before callees and consistent order of callees (module-local functions)"
}

struct Finder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    local_defs: &'a HashMap<LocalDefId, FnMeta>,
    seen: HashSet<LocalDefId>,
    order: Vec<LocalDefId>,
}

impl<'tcx> Visitor<'tcx> for Finder<'_, 'tcx> {
    fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _args) = &ex.kind {
            if let ExprKind::Path(ref qpath) = callee.kind {
                let res = self.cx.qpath_res(qpath, callee.hir_id);
                if let Res::Def(_, def_id) = res {
                    if let Some(local_def_id) = def_id.as_local() {
                        if self.local_defs.contains_key(&local_def_id)
                            && !self.seen.contains(&local_def_id)
                        {
                            self.seen.insert(local_def_id);
                            self.order.push(local_def_id);
                        }
                    }
                }
            }
        }

        // keep traversing
        intravisit::walk_expr(self, ex);
    }
}

#[derive(Clone, Copy)]
struct FnMeta {
    position_number: usize,
    span: Span,
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
            if let ItemKind::Fn { body, .. } = item.kind {
                if item.owner_id.def_id == caller_id {
                    caller_body = Some(body);
                    break;
                }
            }
        }

        caller_body
    }

    fn collect_callees_in_body(
        cx: &LateContext<'_>,
        body_id: BodyId,
        local_defs: &HashMap<LocalDefId, FnMeta>,
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
        must_come_before: HashSet<(LocalDefId, LocalDefId)>,
        functions: HashMap<LocalDefId, FnMeta>,
    ) -> Vec<Violation> {
        let mut violations: Vec<Violation> = must_come_before
            .iter()
            .filter_map(|&(a, b)| {
                let idx_a = functions.get(&a)?.position_number;
                let idx_b = functions.get(&b)?.position_number;
                if idx_a > idx_b {
                    let fn_meta = functions
                        .get(&a)
                        .copied()
                        .expect("Has to be fn meta for function in module");
                    let name_a = cx.tcx.def_path_str(a.to_def_id());
                    let name_b = cx.tcx.def_path_str(b.to_def_id());
                    let violation = Violation {
                        fn_meta,
                        id_first_fn: a,
                        idx_first_fn: idx_a,
                        idx_second_fn: idx_b,
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
                 idx_first_fn: ia1,
                 idx_second_fn: ib1,
                 name_first_fn: name_a1,
                 ..
             },
             Violation {
                 idx_first_fn: ia2,
                 idx_second_fn: ib2,
                 name_first_fn: name_a2,
                 ..
             }| {
                ia1.cmp(ia2)
                    .then(ib1.cmp(ib2))
                    .then(name_a1.as_str().cmp(name_a2.as_str()))
            },
        );

        violations
    }
}

struct Violation {
    idx_first_fn: usize,
    idx_second_fn: usize,
    name_first_fn: String,
    name_second_fn: String,
    id_first_fn: LocalDefId,
    fn_meta: FnMeta,
}

impl<'tcx> LateLintPass<'tcx> for NonTopologicallySortedFunctions {
    // A list of things you might check can be found here:
    // https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html

    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _module_id: HirId) {
        // Collect top-level functions
        let mut def_order: Vec<LocalDefId> = vec![];
        let mut functions: HashMap<LocalDefId, FnMeta> = HashMap::new();
        let mut idx = 0;

        for item_id in module.item_ids {
            let item: &Item<'tcx> = cx.tcx.hir_item(*item_id);
            if let ItemKind::Fn { .. } = item.kind {
                let local_def_id = item.owner_id.def_id;
                let fn_meta = FnMeta {
                    position_number: idx,
                    span: item.span,
                };

                def_order.push(local_def_id);
                functions.insert(local_def_id, fn_meta);

                idx += 1;
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
                    Self::collect_callees_in_body(cx, caller_body_id, &functions);

                must_come_before =
                    Self::build_caller_callee_constraint(caller_id, &callees, must_come_before);
                must_come_before = Self::build_multiple_precedence_rule(&callees, must_come_before);
            }
        }

        let violations = Self::find_violations(cx, must_come_before, functions);
        let mut warned: HashSet<LocalDefId> = HashSet::new();

        for Violation {
            fn_meta,
            id_first_fn,
            name_first_fn,
            name_second_fn,
            ..
        } in violations
        {
            if warned.insert(id_first_fn) {
                cx.span_lint(NON_TOPOLOGICALLY_SORTED_FUNCTIONS, fn_meta.span, |diag| {
                    diag.span_label(fn_meta.span, format!("function `{name_first_fn}` should be defined before `{name_second_fn}`"));
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
