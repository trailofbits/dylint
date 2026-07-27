use crate::{non_local_effect::is_lintable_result, rvalue_places::rvalue_places};
use clippy_utils::{
    paths::{PathLookup, PathNS},
    type_path, value_path,
};
use rustc_hir::def_id::DefId;
use rustc_index::{IndexVec, bit_set::DenseBitSet};
use rustc_lint::LateContext;
use rustc_middle::{
    mir::{
        BasicBlock, Body, Local, Location, Operand, RETURN_PLACE, Rvalue, StatementKind,
        TerminatorKind,
    },
    ty::{Adt, EarlyBinder, Ty},
};
use rustc_span::sym;
use std::collections::VecDeque;

// Backward dataflow analysis that, for each basic block, computes the set of locals whose errors
// (if any) may reach function exit without being consumed by a handling operation. An error is
// considered "handled" if its local is:
// - Returned via the function's return place.
// - Passed as a `Result`-typed argument to another call (e.g., `FromResidual::from_residual`).
// - Passed to `Result::unwrap` or `Result::expect`.
// - Consumed by a call to a function whose return type is `!` (panic, etc.).
//
// The analysis is a hand-rolled worklist rather than an `rustc_mir_dataflow::Analysis` impl
// because modern rustc no longer supports per-edge `SwitchInt` effects in the backward direction
// (see rust-lang/rust#143769), and this analysis relies on applying an edge-specific effect to
// non-error edges of a `Result`/`ControlFlow`-discriminated `SwitchInt`.

/// Runs the backward analysis and returns, for each basic block, the state at the block's end in
/// forward order (equivalent to what the old framework's `seek_to_block_end` would produce).
pub fn analyze<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
) -> IndexVec<BasicBlock, DenseBitSet<Local>> {
    UnhandledErrorsAnalysis { cx, mir }.run()
}

struct UnhandledErrorsAnalysis<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
}

impl<'tcx> UnhandledErrorsAnalysis<'_, 'tcx> {
    fn run(&self) -> IndexVec<BasicBlock, DenseBitSet<Local>> {
        let n_locals = self.mir.local_decls.len();
        let n_blocks = self.mir.basic_blocks.len();

        // state_at_start[B] = state that B propagates to its predecessors (i.e., state at B's
        // start in forward order, after all backward transfer effects have been applied).
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

        IndexVec::from_fn_n(|block| self.state_at_end(block, &state_at_start), n_blocks)
    }

    // Computes the state at the end of `block` (forward order) by joining successor
    // contributions. For a `SwitchInt` on a `Result`/`ControlFlow` discriminant, remove the
    // discriminated local from non-error edges. Other locals must keep flowing: they may contain
    // errors produced before the discriminated operation.
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
                let discriminated_local = self.result_discriminant_local(block, discr);
                let mut joined = DenseBitSet::new_empty(n_locals);
                for (value, target) in targets.iter() {
                    // The discriminant values of `Result::Err` and `ControlFlow::Break` are
                    // both 1. This check should be made more robust.
                    let mut contribution = state_at_start[target].clone();
                    if value != 1
                        && let Some(local) = discriminated_local
                    {
                        contribution.remove(local);
                    }
                    joined.union(&contribution);
                }
                let mut otherwise = state_at_start[targets.otherwise()].clone();
                if let Some(local) = discriminated_local {
                    otherwise.remove(local);
                }
                joined.union(&otherwise);
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

        match &terminator.kind {
            TerminatorKind::Return => {
                // Mark every local as "unhandled going forward from here" except the return
                // place. This seeds the analysis; downstream processing will remove locals as
                // handling operations are found. Note: unlike the original `unhandled_error`
                // lint, we do this regardless of whether the enclosing function returns a
                // `Result`, so that unhandled calls in `() -> ()` functions are also detected.
                state.insert_all();
                state.remove(RETURN_PLACE);
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let changed = state.insert(destination.local);
                if let Some((def_id, substs)) = func.const_fn_def() {
                    if destination.local == RETURN_PLACE
                        && self.cx.tcx.lang_items().from_residual_fn() == Some(def_id)
                        && is_result(self.cx, self.mir.local_decls[RETURN_PLACE].ty)
                    {
                        // Returning a residual reports failure to the caller, so do not treat
                        // errors from earlier calls as silently discarded along this path.
                        state.clear();
                        return state;
                    }
                    let fn_sig = EarlyBinder::bind(self.cx.tcx.fn_sig(def_id).skip_binder())
                        .instantiate(self.cx.tcx, substs);
                    let output = fn_sig.skip_binder().output();
                    if output.is_never() {
                        // Callee doesn't return; assume all locals are handled along this path.
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

        for (statement_index, statement) in basic_block.statements.iter().enumerate().rev() {
            if let StatementKind::Assign(box (place, rvalue)) = &statement.kind
                && state.insert(place.local)
            {
                let location = Location {
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
    fn result_discriminant_local(&self, block: BasicBlock, discr: &Operand<'tcx>) -> Option<Local> {
        let basic_block = &self.mir[block];
        let discr_place = discr.place()?;
        let rvalue = basic_block.statements.iter().rev().find_map(|statement| {
            if let StatementKind::Assign(box (place, rvalue)) = &statement.kind
                && *place == discr_place
            {
                Some(rvalue)
            } else {
                None
            }
        });
        let Rvalue::Discriminant(place) = rvalue? else {
            return None;
        };
        let place_ty = place.ty(&self.mir.local_decls, self.cx.tcx).ty;
        (is_lintable_result(self.cx, place_ty) || is_control_flow_of_result(self.cx, place_ty))
            .then_some(place.local)
    }
}

static RESULT_EXPECT: PathLookup = value_path!(core::result::Result::expect);
static RESULT_UNWRAP: PathLookup = value_path!(core::result::Result::unwrap);

fn is_sink_function(cx: &LateContext<'_>, def_id: DefId) -> bool {
    RESULT_EXPECT.matches(cx, def_id) || RESULT_UNWRAP.matches(cx, def_id)
}

static CONTROL_FLOW: PathLookup = type_path!(core::ops::ControlFlow);

fn is_control_flow_of_result<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let Adt(adt, substs) = ty.kind()
        && CONTROL_FLOW.matches(cx, adt.did())
        && let Some(generic_arg) = substs.iter().next()
        && let Some(inner_ty) = generic_arg.as_type()
    {
        is_result(cx, inner_ty)
    } else {
        false
    }
}

fn is_result<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Result, adt.did()))
}
