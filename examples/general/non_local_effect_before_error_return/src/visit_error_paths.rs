use clippy_utils::ty::implements_trait;
use if_chain::if_chain;
use rustc_hir::intravisit::FnKind;
use rustc_index::bit_set::BitSet;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::mir::{
    BasicBlock, Body, Local, Place, Rvalue, StatementKind, Terminator, TerminatorKind,
    RETURN_PLACE, START_BLOCK,
};

// smoelius: I originally tried to write this analysis using the dataflow framework. But because
// individual paths must be considered, and because of how complicated the state is, this analysis
// didn't seem to be a good fit for the dataflow framework.

// smoelius: There may be an opportunity to optimize this analysis further. Currently, it considers
// all paths that could return an error. By in large, it matters only whether a block appears on
// *some* error path, suggesting that not all error paths need be considered. However, "contributing
// calls" complicate this. That is, a block may contain a call with a non-local effect, but the call
// might not produce a warning because it "contributes" to the error path (e.g., the call returns
// the error that is ultimately returned).
//   This difficulty might be overcome by recording contributing calls at each block, and
// re-exploring a block only if: for each previous exploration of the block, there was at least one
// contributing call not in the current set of contributing calls.
//   However, given how complicated this idea is, and given how complicated the analysis already is,
// I am leaving this to future work. Furthermore, the idea involves recording multiple sets of
// contributing calls at each block, which appears exponential in the number of blocks. Thus, it is
// not clear that this idea would be more efficient than simply considering all error paths.

pub fn visit_error_paths<'tcx>(
    cx: &LateContext<'tcx>,
    fn_kind: FnKind<'tcx>,
    mir: &'tcx Body<'tcx>,
    visitor: impl Fn(&[BasicBlock], &BitSet<BasicBlock>),
) {
    for (index, basic_block) in mir.basic_blocks().iter_enumerated() {
        let terminator = basic_block.terminator();
        if terminator.kind == TerminatorKind::Return {
            let mut guide = Guide::new(cx, fn_kind, mir, &visitor);
            let state = State::new();
            guide.visit_error_paths_to_block_terminator(&state, index);
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    local: Option<Local>,
    possible_variants: BitSet<usize>,
    confirmed_variant: Option<usize>,
}

impl State {
    fn new() -> Self {
        Self {
            local: Some(RETURN_PLACE),
            possible_variants: BitSet::new_filled(2),
            confirmed_variant: None,
        }
    }
    fn on_error_path(&self) -> bool {
        if let Some(variant) = self.confirmed_variant {
            // smoelius: The discriminant values of both `Result::Err` and `ControlFlow::Break` are
            // 1. But this check should be made more robust.
            variant == 1
        } else {
            // smoelius: The next condition intentionally ignores `self.local`. Note that if
            // `self.local` is `None`, then the local was removed and not replaced. One can
            // interpret this to mean: the source of the error was found.
            self.possible_variants.contains(1)
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
    fn remove_possibile_variant(&mut self, variant: usize) {
        if self.confirmed_variant.is_none() {
            self.possible_variants.remove(variant);
        }
    }
    fn set_confirmed_variant(&mut self, variant: usize) {
        if self.possible_variants.contains(variant) {
            self.confirmed_variant = self.confirmed_variant.or(Some(variant));
        }
    }
}

struct Guide<'cx, 'tcx, V> {
    cx: &'cx LateContext<'tcx>,
    fn_kind: FnKind<'tcx>,
    mir: &'tcx Body<'tcx>,
    visitor: V,
    blocks_visited: BitSet<BasicBlock>,
    block_path: Vec<BasicBlock>,
    contributing_calls: BitSet<BasicBlock>,
    work: usize,
}

impl<'cx, 'tcx, V> Guide<'cx, 'tcx, V>
where
    V: Fn(&[BasicBlock], &BitSet<BasicBlock>),
{
    fn new(
        cx: &'cx LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        mir: &'tcx Body<'tcx>,
        visitor: V,
    ) -> Self {
        Self {
            cx,
            fn_kind,
            mir,
            visitor,
            blocks_visited: BitSet::new_empty(mir.basic_blocks().len()),
            block_path: Vec::with_capacity(mir.basic_blocks().len()),
            contributing_calls: BitSet::new_empty(mir.basic_blocks().len()),
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
                StatementKind::Assign(box (assign_place, rvalue)) => {
                    if_chain! {
                        if state.remove_local(assign_place.local);
                        if let Rvalue::Use(rvalue_operand) = rvalue;
                        if let Some(rvalue_place) = rvalue_operand.place();
                        then {
                            state.set_local(rvalue_place.local);
                        }
                    }
                }
                StatementKind::SetDiscriminant {
                    place: box place,
                    variant_index,
                } => {
                    if state.is_local(place.local) {
                        state.set_confirmed_variant(variant_index.as_usize());
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
                (self.visitor)(&self.block_path, &self.contributing_calls);
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
                    if_chain! {
                        if state.remove_local(destination.local);
                        let _ = self.contributing_calls.insert(predecessor);
                        if let Some(arg_place) = is_from_residual_or_try_implementor_method_call(
                            self.cx, self.mir, terminator,
                        );
                        then {
                            state.set_local(arg_place.local);
                        }
                    }
                }
                TerminatorKind::SwitchInt { targets, .. } => {
                    if_chain! {
                        if let Some(rvalue_place) =
                            ends_with_discriminant_switch(self.cx, self.mir, predecessor);
                        if state.is_local(rvalue_place.local);
                        then {
                            for (value, target) in targets.iter() {
                                if target != index {
                                    state.remove_possibile_variant(value as usize);
                                }
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
        let work_limit = work_limit();
        if self.work >= work_limit {
            let name = match self.fn_kind {
                FnKind::ItemFn(ident, _, _) | FnKind::Method(ident, _) => format!("`{}`", ident),
                FnKind::Closure => "closure".to_owned(),
            };
            self.cx.sess().warn(&format!(
                "reached work limit ({}) while checking {}; set `{}` to override",
                work_limit,
                name,
                env!("CARGO_PKG_NAME").to_uppercase() + WORK_LIMIT_SUFFIX
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
    if_chain! {
        if let TerminatorKind::Call { func, args, .. } = &terminator.kind;
        if let Some((def_id, _)) = func.const_fn_def();
        if let [arg, ..] = args.as_slice();
        if let Some(arg_place) = arg.place();
        let _ = if Some(def_id) == cx.tcx.lang_items().from_residual_fn() {
            return Some(arg_place);
        };
        if let Some(assoc_item) = cx.tcx.opt_associated_item(def_id);
        if assoc_item.fn_has_self_parameter;
        if let Some(try_trait_def_id) = cx.tcx.lang_items().try_trait();
        let arg_place_ty = arg_place.ty(&mir.local_decls, cx.tcx);
        // smoelius: It appears that all type parameters must be substituted for, or else
        // `implements_trait` returns false.
        if implements_trait(cx, arg_place_ty.ty, try_trait_def_id, &[]);
        then {
            Some(arg_place)
        } else {
            None
        }
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
    if_chain! {
        if let TerminatorKind::SwitchInt { discr, .. } = &terminator.kind;
        if let Some(discr_place) = discr.place();
        then {
            basic_block.statements.iter().rev().find_map(|statement| {
                if_chain! {
                    if let StatementKind::Assign(box (
                        assign_place,
                        Rvalue::Discriminant(rvalue_place),
                    )) = &statement.kind;
                    if *assign_place == discr_place;
                    then {
                        Some(*rvalue_place)
                    } else {
                        None
                    }
                }
            })
        } else {
            None
        }
    }
}

const WORK_LIMIT_DEFAULT: usize = 500_000;
const WORK_LIMIT_SUFFIX: &str = "_WORK_LIMIT";

#[must_use]
fn work_limit() -> usize {
    let key = env!("CARGO_PKG_NAME").to_uppercase() + WORK_LIMIT_SUFFIX;
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(WORK_LIMIT_DEFAULT)
}
