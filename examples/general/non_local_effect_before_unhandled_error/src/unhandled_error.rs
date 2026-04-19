use super::rvalue_places;
use clippy_utils::{
    diagnostics::span_lint,
    paths::{PathLookup, PathNS},
    type_path, value_path,
};
use rustc_hir::{Body, FnDecl, def_id::DefId, def_id::LocalDefId, intravisit::FnKind};
use rustc_index::{Idx, IndexVec, bit_set::DenseBitSet};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        self, BasicBlock, Local, Operand, RETURN_PLACE, Rvalue, StatementKind, TerminatorKind,
        pretty::MirWriter,
    },
    ty::{self, EarlyBinder, Ty},
};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::{Span, sym};
use std::collections::VecDeque;

declare_lint! {
    /// **What it does:**
    ///
    /// **Why is this bad?**
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // example code where a warning is issued
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code that does not raise a warning
    /// ```
    pub UNHANDLED_ERROR,
    Warn,
    "description goes here"
}

declare_lint_pass!(UnhandledError => [UNHANDLED_ERROR]);

// smoelius: This lint has essentially two steps:
// 1. Perform a source-to-sink analysis. The sinks are:
//    * the return place if the return type is a `Result`
//    * all call arguments whose type is a `Result`
//    * all locals wherever there is a call to a function that does not return
//    Propagate backward to find all locals that flow to these sinks.
// 2. Find calls whose return value is of type `Result` and does not flow to a sink. Emit a lint
//    warning for such calls.
//
// The analysis is expressed as a hand-rolled backward worklist because the `SwitchInt` edge-effect
// hook this lint relies on was removed from `rustc_mir_dataflow` in rust-lang/rust#143769. See the
// comment on `join_successors` for the specific per-edge transform.

impl<'tcx> LateLintPass<'tcx> for UnhandledError {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: LocalDefId,
    ) {
        let local_def_id = cx.tcx.hir_body_owner_def_id(body.id());

        let mir = cx.tcx.optimized_mir(local_def_id.to_def_id());

        let returns_result = is_result(cx, cx.typeck_results().expr_ty(body.value));

        if enabled("UE_DUMP_MIR") {
            let writer = MirWriter::new(cx.tcx);
            writer.write_mir_fn(mir, &mut std::io::stdout()).unwrap();
        }

        // smoelius: Step 1

        let analysis = UnhandledErrorsAnalysis {
            cx,
            mir,
            returns_result,
        };

        // state_at_start[B] = state that B propagates to its predecessors (i.e., state at the
        // block's start in forward order, after all backward transfer effects).
        let state_at_start = analysis.run();

        if enabled("UE_DUMP_ANALYSIS") {
            for (index, _) in mir.basic_blocks.iter_enumerated() {
                let before = state_at_start[index].clone();
                let after = analysis.state_at_end(index, &state_at_start);
                println!("{index:?}: {:?} -> {:?}", invert(&before), invert(&after));
            }
        }

        // smoelius: Step 2

        for (index, basic_block) in mir.basic_blocks.iter_enumerated() {
            let terminator = basic_block.terminator();
            if let TerminatorKind::Call {
                destination,
                fn_span,
                ..
            } = &terminator.kind
                && is_result(cx, destination.ty(&mir.local_decls, cx.tcx).ty)
            {
                let state = analysis.state_at_end(index, &state_at_start);
                if state.contains(destination.local) {
                    span_lint(
                        cx,
                        UNHANDLED_ERROR,
                        fn_span.data().span(),
                        "this call's result may not be handled along all error paths",
                    );
                }
            }
        }
    }
}

struct UnhandledErrorsAnalysis<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    mir: &'tcx mir::Body<'tcx>,
    returns_result: bool,
}

// smoelius: The `UnhandledErrors` domain stores the locals that are unhandled at a given point in
// the program. It would seem more intuitive to me to store the handled locals instead of the
// unhandled ones. I didn't do this for the following reason. An error is handled if-and-only-if it
// is handled along all error paths. Thus, if we were to store the handled locals, then at
// `SwitchInt`s, we would have to compute the intersection of its block's successor states. But a
// union-style join is more natural in this framework, so we store unhandled errors and compute
// their union.
//   A couple notable points:
// * At returns, only the returned local is considered handled; all other locals are considered
//   unhandled.
// * At function calls that do not return (e.g., panics), all locals are considered handled.

impl<'tcx> UnhandledErrorsAnalysis<'_, 'tcx> {
    fn run(&self) -> IndexVec<BasicBlock, DenseBitSet<Local>> {
        let n_locals = self.mir.local_decls.len();
        let n_blocks = self.mir.basic_blocks.len();

        let mut state_at_start: IndexVec<BasicBlock, DenseBitSet<Local>> =
            IndexVec::from_fn_n(|_| DenseBitSet::new_empty(n_locals), n_blocks);

        let mut worklist: VecDeque<BasicBlock> = VecDeque::with_capacity(n_blocks);
        let mut in_worklist = DenseBitSet::new_empty(n_blocks);
        for (index, _) in self.mir.basic_blocks.iter_enumerated() {
            worklist.push_back(index);
            in_worklist.insert(index);
        }

        while let Some(block) = worklist.pop_front() {
            in_worklist.remove(block);

            let end = self.state_at_end(block, &state_at_start);
            let start = self.apply_backward_transfer(block, end);

            if start != state_at_start[block] {
                state_at_start[block] = start;
                for &pred in &self.mir.basic_blocks.predecessors()[block] {
                    if !in_worklist.contains(pred) {
                        worklist.push_back(pred);
                        in_worklist.insert(pred);
                    }
                }
            }
        }

        state_at_start
    }

    // Computes the state at the end of `block` (forward order) by joining successor contributions.
    // For `SwitchInt` on a `Result`/`ControlFlow` discriminant, non-`Err` edges contribute the
    // empty set (i.e., they are effectively pruned) — this is the per-edge effect that used to be
    // expressed via `apply_switch_int_edge_effects`.
    fn state_at_end(
        &self,
        block: BasicBlock,
        state_at_start: &IndexVec<BasicBlock, DenseBitSet<Local>>,
    ) -> DenseBitSet<Local> {
        let n_locals = self.mir.local_decls.len();
        let terminator = &self.mir[block].terminator().kind;

        match terminator {
            TerminatorKind::Return | TerminatorKind::Unreachable => {
                DenseBitSet::new_empty(n_locals)
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let prune = self.is_result_discriminant_switch(block, discr);
                let mut joined = DenseBitSet::new_empty(n_locals);
                for (value, target) in targets.iter() {
                    // smoelius: Ignore `Ok` and `otherwise` edges, since we only care what
                    // happens to a `Result` when it is an `Err`. Accomplish this by contributing
                    // the empty set on those edges.
                    //   The discriminant values of `Result::Err` and `ControlFlow::Break` are
                    // both 1. But this check should be made more robust.
                    if prune && value != 1 {
                        continue;
                    }
                    joined.union(&state_at_start[target]);
                }
                if !prune {
                    joined.union(&state_at_start[targets.otherwise()]);
                }
                joined
            }
            _ => {
                let mut joined = DenseBitSet::new_empty(n_locals);
                for succ in self.mir[block].terminator().successors() {
                    joined.union(&state_at_start[succ]);
                }
                joined
            }
        }
    }

    // Transforms `state` from "state at end of block" to "state at start of block" by applying
    // the backward effects of the terminator and then the statements in reverse.
    fn apply_backward_transfer(
        &self,
        block: BasicBlock,
        mut state: DenseBitSet<Local>,
    ) -> DenseBitSet<Local> {
        let basic_block = &self.mir[block];
        let terminator = basic_block.terminator();

        // Terminator effect (equivalent to the old `apply_before_terminator_effect`).
        match &terminator.kind {
            TerminatorKind::Return => {
                if self.returns_result {
                    state.insert_all();
                    state.remove(RETURN_PLACE);
                }
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let changed = state.insert(destination.local);
                if let Some((def_id, substs)) = func.const_fn_def() {
                    let fn_sig = EarlyBinder::bind(self.cx.tcx.fn_sig(def_id).skip_binder())
                        .instantiate(self.cx.tcx, substs);
                    let output = fn_sig.skip_binder().output();
                    if output.is_never() {
                        // smoelius: If the callee doesn't return, assume all locals are handled.
                        state.clear();
                    } else if changed || is_sink_function(self.cx, def_id) {
                        let inputs = fn_sig.skip_binder().inputs();
                        if inputs.len() == args.len() {
                            for (input, arg) in inputs.iter().zip(args.iter()) {
                                if is_result(self.cx, *input)
                                    && let Some(place) = arg.node.place()
                                {
                                    state.remove(place.local);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Statements in reverse.
        for (statement_index, statement) in basic_block.statements.iter().enumerate().rev() {
            if let StatementKind::Assign(box (place, rvalue)) = &statement.kind
                && state.insert(place.local)
            {
                let location = mir::Location {
                    block,
                    statement_index,
                };
                for rv_place in rvalue_places(rvalue, location) {
                    state.remove(rv_place.local);
                }
            }
        }

        state
    }

    // Example:
    //     bb1: {
    //         _3 = discriminant(_2);
    //         switchInt(move _3) -> [0_isize: bb4, 1_isize: bb2, otherwise: bb3];
    //     }
    fn is_result_discriminant_switch(&self, block: BasicBlock, discr: &Operand<'tcx>) -> bool {
        let basic_block = &self.mir[block];
        let Some(discr_place) = discr.place() else {
            return false;
        };
        let rvalue = basic_block.statements.iter().rev().find_map(|statement| {
            if let StatementKind::Assign(box (place, rvalue)) = &statement.kind
                && *place == discr_place
            {
                Some(rvalue)
            } else {
                None
            }
        });
        let Some(Rvalue::Discriminant(place)) = rvalue else {
            return false;
        };
        let place_ty = place.ty(&self.mir.local_decls, self.cx.tcx).ty;
        is_result(self.cx, place_ty) || is_control_flow_result(self.cx, place_ty)
    }
}

static RESULT_EXPECT: PathLookup = value_path!(core::result::Result::expect);
static RESULT_UNWRAP: PathLookup = value_path!(core::result::Result::unwrap);

fn is_sink_function(cx: &LateContext<'_>, def_id: DefId) -> bool {
    RESULT_EXPECT.matches(cx, def_id) || RESULT_UNWRAP.matches(cx, def_id)
}

fn is_result<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Result, adt.did()))
}

static CONTROL_FLOW: PathLookup = type_path!(core::ops::ControlFlow);

fn is_control_flow_result<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::Adt(adt, substs) = ty.kind()
        && CONTROL_FLOW.matches(cx, adt.did())
        && let Some(generic_arg) = substs.iter().next()
        && let Some(inner_ty) = generic_arg.as_type()
    {
        is_result(cx, inner_ty)
    } else {
        false
    }
}

#[must_use]
fn enabled(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| value != "0")
}

fn invert(state: &DenseBitSet<Local>) -> Vec<Local> {
    (0..state.domain_size())
        .map(Local::new)
        .filter(|&l| !state.contains(l))
        .collect()
}
