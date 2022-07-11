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
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        pretty::write_mir_fn, BasicBlock, BasicBlockData, Body, Mutability, Operand, Place,
        ProjectionElem, Rvalue, Statement, StatementKind, TerminatorKind,
    },
    ty,
};
use rustc_span::{sym, Span};

mod visit_error_paths;
use visit_error_paths::visit_error_paths;

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
            for &index in path {
                let basic_block = &mir.basic_blocks()[index];

                if_chain! {
                    if !contributing_calls.contains(index);
                    if let Some(call_span) = is_call_with_mut_ref(cx, mir, index);
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
    index: BasicBlock,
) -> Option<Span> {
    let basic_block = &mir[index];
    let terminator = basic_block.terminator();
    if_chain! {
        if let TerminatorKind::Call {
            func,
            args,
            fn_span,
            ..
        } = &terminator.kind;
        if let Some((def_id, _)) = func.const_fn_def();
        let fn_sig = cx.tcx.fn_sig(def_id).skip_binder();
        if args
            .iter()
            .zip(fn_sig.inputs())
            .any(|(arg, &input_ty)| is_mut_ref_arg(cx, mir, basic_block, arg, input_ty));
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
    _cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    basic_block: &BasicBlockData<'tcx>,
    arg: &Operand<'tcx>,
    input_ty: ty::Ty,
) -> bool {
    if_chain! {
        if let Operand::Copy(operand_place) | Operand::Move(operand_place) = arg;
        let body_args = 1..=mir.arg_count;
        if body_args.contains(&operand_place.local.as_usize())
            || basic_block.statements.iter().rev().any(|statement| {
                if_chain! {
                    if let StatementKind::Assign(box (assign_place, rvalue)) = &statement.kind;
                    if assign_place == operand_place;
                    if let Rvalue::Use(
                        Operand::Copy(rvalue_place) | Operand::Move(rvalue_place),
                    ) | Rvalue::Ref(_, _, rvalue_place) = rvalue;
                    if body_args.contains(&rvalue_place.local.as_usize());
                    then {
                        true
                    } else {
                        false
                    }
                }
            });
        if matches!(input_ty.kind(), ty::Ref(_, _, Mutability::Mut));
        then {
            true
        } else {
            false
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
