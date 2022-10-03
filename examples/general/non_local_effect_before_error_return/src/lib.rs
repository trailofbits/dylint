#![feature(box_patterns)]
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::intravisit::FnKind;
use rustc_index::bit_set::BitSet;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        pretty::write_mir_fn, BasicBlock, BasicBlockData, Body, Location, Mutability, Operand,
        Place, ProjectionElem, Statement, StatementKind, TerminatorKind,
    },
    ty,
};
use rustc_span::{sym, Span};

mod visit_error_paths;
use visit_error_paths::visit_error_paths;

mod rvalue_places;
use rvalue_places::rvalue_places;

dylint_linting::declare_late_lint! {
    /// **What it does:** Checks for non-local effects (e.g., assignments to mutable references)
    /// before return of an error.
    ///
    /// **Why is this bad?** Functions that make changes to the program state before returning an
    /// error are difficult to reason about. Generally speaking, if a function returns an error, it
    /// should be as though the function was never called.
    ///
    /// **Known problems:**
    /// * The search strategy is exponential in the number of blocks in a function body. To help
    ///   deal with complex bodies, the lint includes a "work limit" (see "Options" below).
    /// * Errors in loops are not handled properly.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # struct Account { balance: i64 }
    /// # struct InsufficientBalance;
    /// impl Account {
    ///     fn withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
    ///         self.balance -= amount;
    ///         if self.balance < 0 {
    ///             return Err(InsufficientBalance);
    ///         }
    ///         Ok(self.balance)
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// # struct Account { balance: i64 }
    /// # struct InsufficientBalance;
    /// impl Account {
    ///     fn withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
    ///         let new_balance = self.balance - amount;
    ///         if new_balance < 0 {
    ///             return Err(InsufficientBalance);
    ///         }
    ///         self.balance = new_balance;
    ///         Ok(self.balance)
    ///     }
    /// }
    /// ```
    ///
    /// **Options:**
    /// `NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN_WORK_LIMIT` (default 500000): When exploring a
    /// function body, the maximum number of times the search path is extended. Setting this to a
    /// higher number allows more bodies to be explored exhaustively, but at the expense of greater
    /// runtime.
    pub NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
    Warn,
    "non-local effects before return of an error"
}

impl<'tcx> LateLintPass<'tcx> for NonLocalEffectBeforeErrorReturn {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'_>,
        body: &'tcx rustc_hir::Body<'_>,
        span: Span,
        _: rustc_hir::HirId,
    ) {
        if span.from_expansion() {
            return;
        }

        if !is_result(cx, cx.typeck_results().expr_ty(&body.value)) {
            return;
        }

        let local_def_id = cx.tcx.hir().body_owner_def_id(body.id());

        let mir = cx.tcx.optimized_mir(local_def_id.to_def_id());

        if enabled("DEBUG_MIR") {
            write_mir_fn(cx.tcx, mir, &mut |_, _| Ok(()), &mut std::io::stdout()).unwrap();
        }

        visit_error_paths(cx, fn_kind, mir, |path, contributing_calls| {
            // smoelius: The path is from a return to the start block.
            for (i, &index) in path.iter().enumerate() {
                let basic_block = &mir.basic_blocks[index];

                if_chain! {
                    if !contributing_calls.contains(index);
                    if let Some(call_span) = is_call_with_mut_ref(cx, mir, &path[i..]);
                    then {
                        span_lint(
                            cx,
                            NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
                            call_span,
                            "call with mutable reference before error return",
                        );
                    }
                }

                for statement in basic_block.statements.iter().rev() {
                    if let Some(assign_span) = is_deref_assign(cx, mir, statement) {
                        span_lint(
                            cx,
                            NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
                            assign_span,
                            "assignment to dereference before error return",
                        );
                    }
                }
            }
        });
    }
}

fn is_result(cx: &LateContext<'_>, ty: ty::Ty) -> bool {
    if let ty::Adt(adt, _) = ty.kind() {
        cx.tcx.is_diagnostic_item(sym::Result, adt.did())
    } else {
        false
    }
}

fn is_call_with_mut_ref<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    path: &[BasicBlock],
) -> Option<Span> {
    let index = path[0];
    let basic_block = &mir[index];
    let terminator = basic_block.terminator();
    if_chain! {
        if let TerminatorKind::Call { args, fn_span, .. } = &terminator.kind;
        if args
            .iter()
            .any(|arg| is_mut_ref_arg(cx, mir, path, basic_block, arg));
        then {
            Some(*fn_span)
        } else {
            None
        }
    }
}

// smoelius: From: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.Body.html#structfield.local_decls
// The first local is the return value pointer, followed by `arg_count` locals for the function arguments, ...
//                                                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
fn is_mut_ref_arg<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    path: &[BasicBlock],
    basic_block: &BasicBlockData<'tcx>,
    arg: &Operand<'tcx>,
) -> bool {
    let body_args = 1..=mir.arg_count;

    let mut locals = BitSet::new_empty(mir.local_decls.len());
    if let Some(arg_place) = mut_ref_operand_place(cx, mir, arg) {
        locals.insert(arg_place.local)
    } else {
        return false;
    };

    for (i, &index) in path.iter().enumerate() {
        if body_args.clone().any(|arg| locals.contains(arg.into())) {
            return true;
        }

        if i != 0 {
            let basic_block = &mir[index];
            let terminator = basic_block.terminator();
            // smoelius: If a call assigns to a followed local, then for each argument that is a
            // mutable reference, assume it refers to the same underlying memory as the local.
            if_chain! {
                if let TerminatorKind::Call {
                    destination, args, ..
                } = &terminator.kind;
                if locals.remove(destination.local);
                then {
                    for arg in args {
                        if let Some(arg_place) = mut_ref_operand_place(cx, mir, arg) {
                            locals.insert(arg_place.local);
                        }
                    }
                }
            }
        }

        for (statement_index, statement) in basic_block.statements.iter().enumerate().rev() {
            if body_args.clone().any(|arg| locals.contains(arg.into())) {
                return true;
            }

            if_chain! {
                if let StatementKind::Assign(box (assign_place, rvalue)) = &statement.kind;
                if locals.remove(assign_place.local);
                if let [rvalue_place, ..] = rvalue_places(
                    rvalue,
                    Location {
                        block: index,
                        statement_index,
                    },
                )
                .as_slice();
                then {
                    locals.insert(rvalue_place.local);
                }
            }
        }
    }

    body_args.clone().any(|arg| locals.contains(arg.into()))
}

fn mut_ref_operand_place<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    operand: &Operand<'tcx>,
) -> Option<Place<'tcx>> {
    if_chain! {
        if let Some(operand_place) = operand.place();
        if matches!(
            operand_place.ty(&mir.local_decls, cx.tcx).ty.kind(),
            ty::Ref(_, _, Mutability::Mut)
        );
        then {
            Some(operand_place)
        } else {
            None
        }
    }
}

fn is_deref_assign<'tcx>(
    _cx: &LateContext<'tcx>,
    _mir: &'tcx Body<'tcx>,
    statement: &Statement,
) -> Option<Span> {
    if_chain! {
        if let StatementKind::Assign(box (Place { projection, .. }, _)) = &statement.kind;
        if projection.iter().any(|elem| elem == ProjectionElem::Deref);
        then {
            Some(statement.source_info.span)
        } else {
            None
        }
    }
}

#[must_use]
fn enabled(opt: &str) -> bool {
    let key = env!("CARGO_PKG_NAME").to_uppercase() + "_" + opt;
    std::env::var(key).map_or(false, |value| value != "0")
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
