use clippy_utils::ty::implements_trait;
use rustc_abi::VariantIdx;
use rustc_hir::intravisit::FnKind;
use rustc_index::bit_set::DenseBitSet;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::{
    mir::{
        AggregateKind, BasicBlock, Body, Local, Place, ProjectionElem, RETURN_PLACE, Rvalue,
        START_BLOCK, StatementKind, Terminator, TerminatorKind,
    },
    ty::{AdtDef, TyCtxt},
};
use rustc_span::Span;

// smoelius: I originally tried to write this analysis using the dataflow framework. But because
// individual paths must be considered, and because of how complicated the state is, this analysis
// didn't seem to be a good fit for the dataflow framework.

// smoelius: There may be an opportunity to optimize this analysis further. Currently, it considers
// all paths that could return an error. By in large, it matters only whether a block appears on
// *some* error path, suggesting that not all error paths need be considered. However, "contributing
// calls" complicate this. That is, a block may contain a call with a non-local effect, but the call
// might not produce a warning because it "contributes" to the error path (e.g., the call returns
// the error that is ultimately returned).
//
// This difficulty might be overcome by recording contributing calls at each block, and re-exploring
// a block only if: for each previous exploration of the block, there was at least one contributing
// call not in the current set of contributing calls.
//
// However, given how complicated this idea is, and given how complicated the analysis already is, I
// am leaving this to future work. Furthermore, the idea involves recording multiple sets of
// contributing calls at each block, which appears exponential in the number of blocks. Thus, it is
// not clear that this idea would be more efficient than simply considering all error paths.

pub fn visit_error_paths<'tcx>(
    work_limit: u64,
    cx: &LateContext<'tcx>,
    fn_kind: FnKind<'tcx>,
    mir: &'tcx Body<'tcx>,
    visitor: impl Fn(&[BasicBlock], &DenseBitSet<BasicBlock>, Option<Span>),
) {
    for (index, basic_block) in mir.basic_blocks.iter_enumerated() {
        let terminator = basic_block.terminator();
        if terminator.kind == TerminatorKind::Return {
            let mut guide = Guide::new(work_limit, cx, fn_kind, mir, &visitor);
            let state = State::new();
            guide.visit_error_paths_to_block_terminator(&state, index);
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    local: Option<Local>,
    possible_variants: DenseBitSet<VariantIdx>,
    confirmed_variant: Option<VariantIdx>,
    span: Option<Span>,
}

impl State {
    fn new() -> Self {
        Self {
            local: Some(RETURN_PLACE),
            possible_variants: DenseBitSet::new_filled(2),
            confirmed_variant: None,
            span: None,
        }
    }
    fn on_error_path(&self) -> bool {
        if let Some(variant) = self.confirmed_variant {
            // smoelius: The variant indices of both `Result::Err` and `ControlFlow::Break` are
            // 1. But this check should be made more robust.
            variant == VariantIdx::from_u32(1)
        } else {
            // smoelius: The next condition intentionally ignores `self.local`. Note that if
            // `self.local` is `None`, then the local was removed and not replaced. One can
            // interpret this to mean: the source of the error was found.
            self.possible_variants.contains(VariantIdx::from_u32(1))
        }
    }
    fn is_local(&self, local: Local) -> bool {
        self.local == Some(local)
    }
    fn remove_local(&mut self, local: Local) -> bool {
        if self.is_local(local) {
            self.local = None;
            true
        } else {
            false
        }
    }
    fn set_local(&mut self, local: Local) {
        assert!(self.local.is_none());
        self.local = Some(local);
    }
    fn remove_possible_variant(&mut self, variant: VariantIdx, span: Span) {
        if self.confirmed_variant.is_none() {
            self.possible_variants.remove(variant);
            self.span = self.span.or(Some(span));
        }
    }
    fn set_confirmed_variant(&mut self, variant: VariantIdx, span: Span) {
        if self.possible_variants.contains(variant) {
            // smoelius: Once the variant is confirmed, there is no point in tracking the local.
            self.local = None;
            self.confirmed_variant = self.confirmed_variant.or(Some(variant));
            self.span = self.span.or(Some(span));
        }
    }
}

struct Guide<'cx, 'tcx, V> {
    work_limit: u64,
    cx: &'cx LateContext<'tcx>,
    fn_kind: FnKind<'tcx>,
    mir: &'tcx Body<'tcx>,
    visitor: V,
    blocks_visited: DenseBitSet<BasicBlock>,
    block_path: Vec<BasicBlock>,
    contributing_calls: DenseBitSet<BasicBlock>,
    work: u64,
}

impl<'cx, 'tcx, V> Guide<'cx, 'tcx, V>
where
    V: Fn(&[BasicBlock], &DenseBitSet<BasicBlock>, Option<Span>),
{
    fn new(
        work_limit: u64,
        cx: &'cx LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        mir: &'tcx Body<'tcx>,
        visitor: V,
    ) -> Self {
        Self {
            work_limit,
            cx,
            fn_kind,
            mir,
            visitor,
            blocks_visited: DenseBitSet::new_empty(mir.basic_blocks.len()),
            block_path: Vec::with_capacity(mir.basic_blocks.len()),
            contributing_calls: DenseBitSet::new_empty(mir.basic_blocks.len()),
            work: 0,
        }
    }

    fn visit_error_paths_to_block_terminator(&mut self, state: &State, index: BasicBlock) {
        assert!(!self.blocks_visited.contains(index));
        if self.work() {
            return;
        }
        self.blocks_visited.insert(index);
        self.block_path.push(index);
        let mut state = state.clone();
        let basic_block = &self.mir[index];
        for statement in basic_block.statements.iter().rev() {
            match &statement.kind {
                StatementKind::Assign(box (
                    place,
                    Rvalue::Aggregate(box AggregateKind::Adt(_, variant_index, _, _, _), _),
                ))
                | StatementKind::SetDiscriminant {
                    place: box place,
                    variant_index,
                } => {
                    if state.is_local(place.local) {
                        state.set_confirmed_variant(*variant_index, statement.source_info.span);
                    }
                }
                StatementKind::Assign(box (assign_place, rvalue)) => {
                    if state.remove_local(assign_place.local)
                        && let Rvalue::Use(rvalue_operand) = rvalue
                        && let Some(rvalue_place) = rvalue_operand.place()
                    {
                        state.set_local(rvalue_place.local);
                    }
                }
                _ => {}
            }
        }
        // smoelius: Don't recurse unnecessarily.
        if state.on_error_path() {
            self.visit_error_paths_to_block_entry(&state, index);
        }
        assert!(self.block_path.pop() == Some(index));
        assert!(self.blocks_visited.remove(index));
    }

    fn visit_error_paths_to_block_entry(&mut self, state: &State, index: BasicBlock) {
        assert!(self.blocks_visited.contains(index));
        if index == START_BLOCK {
            if state.on_error_path() {
                (self.visitor)(&self.block_path, &self.contributing_calls, state.span);
            }
            return;
        }
        for &predecessor in &self.mir.basic_blocks.predecessors()[index] {
            if self.blocks_visited.contains(predecessor) {
                continue;
            }
            let mut state = state.clone();
            let basic_block = &self.mir[predecessor];
            let terminator = basic_block.terminator();
            match &terminator.kind {
                TerminatorKind::Return => {
                    unreachable!();
                }
                TerminatorKind::Call { destination, .. } => {
                    if state.remove_local(destination.local)
                        && let _ = self.contributing_calls.insert(predecessor)
                        && let Some(arg_place) = is_from_residual_or_try_implementor_method_call(
                            self.cx, self.mir, terminator,
                        )
                    {
                        state.set_local(arg_place.local);
                    }
                }
                TerminatorKind::SwitchInt { targets, .. } => {
                    if let Some(rvalue_place) =
                            ends_with_discriminant_switch(self.cx, self.mir, predecessor)
                        // smoelius: The next list may need to expand beyond just
                        // `ProjectionElem::Downcast`.
                        && !rvalue_place
                            .projection
                            .iter()
                            .any(|elem| matches!(elem, ProjectionElem::Downcast(_, _)))
                        && state.is_local(rvalue_place.local)
                    {
                        let adt_def = self.mir.local_decls[RETURN_PLACE].ty.ty_adt_def().unwrap();
                        for (value, target) in targets.iter() {
                            if target != index {
                                let variant_idx =
                                    variant_for_discriminant(self.cx.tcx, adt_def, value).unwrap();
                                state.remove_possible_variant(
                                    variant_idx,
                                    terminator.source_info.span,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
            // smoelius: Don't recurse unnecessarily
            if state.on_error_path() {
                self.visit_error_paths_to_block_terminator(&state, predecessor);
            }
            self.contributing_calls.remove(predecessor);
        }
    }

    // Emits a warning and returns true if work limit has been reached.
    fn work(&mut self) -> bool {
        if self.work >= self.work_limit {
            let name = match self.fn_kind {
                FnKind::ItemFn(ident, _, _) | FnKind::Method(ident, _) => format!("`{ident}`"),
                FnKind::Closure => "closure".to_owned(),
            };
            self.cx.sess().dcx().warn(format!(
                "reached work limit ({}) while checking {}; set `{}.work_limit` in `dylint.toml` \
                 to override",
                self.work_limit,
                name,
                env!("CARGO_PKG_NAME")
            ));
            return true;
        }
        self.work += 1;
        false
    }
}

fn is_from_residual_or_try_implementor_method_call<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    terminator: &Terminator<'tcx>,
) -> Option<Place<'tcx>> {
    if let TerminatorKind::Call { func, args, .. } = &terminator.kind
        && let Some((def_id, _)) = func.const_fn_def()
        && let [arg, ..] = args.as_ref()
        && let Some(arg_place) = arg.node.place()
        && {
            if cx.tcx.lang_items().from_residual_fn() == Some(def_id) {
                return Some(arg_place);
            }
            true
        }
        && let Some(assoc_item) = cx.tcx.opt_associated_item(def_id)
        && assoc_item.fn_has_self_parameter
        && let Some(try_trait_def_id) = cx.tcx.lang_items().try_trait()
        && let arg_place_ty = arg_place.ty(&mir.local_decls, cx.tcx)
        // smoelius: It appears that all type parameters must be substituted for, or else
        // `implements_trait` returns false.
        && implements_trait(cx, arg_place_ty.ty, try_trait_def_id, &[])
    {
        Some(arg_place)
    } else {
        None
    }
}

// smoelius: Example:
//     bb1: {
//         _3 = discriminant(_2);
//         switchInt(move _3) -> [0_isize: bb4, 1_isize: bb2, otherwise: bb3];
//     }
fn ends_with_discriminant_switch<'tcx>(
    _cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    index: BasicBlock,
) -> Option<Place<'tcx>> {
    let basic_block = &mir[index];
    let terminator = basic_block.terminator();
    if let TerminatorKind::SwitchInt { discr, .. } = &terminator.kind
        && let Some(discr_place) = discr.place()
    {
        basic_block.statements.iter().rev().find_map(|statement| {
            if let StatementKind::Assign(box (assign_place, Rvalue::Discriminant(rvalue_place))) =
                &statement.kind
                && *assign_place == discr_place
            {
                Some(*rvalue_place)
            } else {
                None
            }
        })
    } else {
        None
    }
}

fn variant_for_discriminant<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt_def: AdtDef<'tcx>,
    value: u128,
) -> Option<VariantIdx> {
    adt_def
        .discriminants(tcx)
        .find_map(|(i, discr)| if value == discr.val { Some(i) } else { None })
}
