#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_abi;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_then, match_def_path};
use rustc_errors::Diag;
use rustc_hir::{def_id::LocalDefId, intravisit::FnKind};
use rustc_index::bit_set::DenseBitSet;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        BasicBlock, Body, ConstOperand, Local, Location, Mutability, Operand, Place,
        ProjectionElem, Rvalue, Statement, StatementKind, TerminatorKind,
        pretty::{PrettyPrintMirOptions, write_mir_fn},
    },
    ty,
};
use rustc_span::{Span, sym};
use serde::Deserialize;

mod visit_error_paths;
use visit_error_paths::visit_error_paths;

mod rvalue_places;
use rvalue_places::rvalue_places;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for non-local effects (e.g., assignments to mutable references) before return of an
    /// error.
    ///
    /// ### Why is this bad?
    ///
    /// Functions that make changes to the program state before returning an error are difficult to
    /// reason about. Generally speaking, if a function returns an error, it should be as though the
    /// function was never called.
    ///
    /// ### Known problems
    ///
    /// - The search strategy is exponential in the number of blocks in a function body. To help
    ///   deal with complex bodies, the lint includes a "work limit" (see "Configuration" below).
    /// - Errors in loops are not handled properly.
    ///
    /// ### Example
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
    ///
    /// Use instead:
    ///
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
    /// ### Configuration
    ///
    /// - `public_only: bool` (default `true`): Whether to check only publicly accessible functions.
    /// - `work_limit: u64` (default 500000): When exploring a function body, the maximum number of
    ///   times the search path is extended. Setting this to a higher number allows more bodies to
    ///   be explored exhaustively, but at the expense of greater runtime.
    pub NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
    Warn,
    "non-local effects before return of an error",
    NonLocalEffectBeforeErrorReturn::new()
}

#[derive(Deserialize)]
struct Config {
    public_only: Option<bool>,
    work_limit: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            public_only: Some(true),
            work_limit: Some(500_000),
        }
    }
}

struct NonLocalEffectBeforeErrorReturn {
    config: Config,
}

impl NonLocalEffectBeforeErrorReturn {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for NonLocalEffectBeforeErrorReturn {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'_>,
        body: &'tcx rustc_hir::Body<'_>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }

        if self
            .config
            .public_only
            .unwrap_or_else(|| Config::default().public_only.unwrap())
            && !cx.effective_visibilities.is_exported(local_def_id)
        {
            return;
        }

        // smoelius: Ignore async functions (at least for now).
        if in_async_function(cx.tcx, body.id().hir_id) {
            return;
        }

        if !is_result(cx, cx.typeck_results().expr_ty(body.value)) {
            return;
        }

        let local_def_id = cx.tcx.hir_body_owner_def_id(body.id());

        let mir = cx.tcx.optimized_mir(local_def_id.to_def_id());

        if enabled("DEBUG_MIR") {
            let options = PrettyPrintMirOptions::from_cli(cx.tcx);
            write_mir_fn(
                cx.tcx,
                mir,
                &mut |_, _| Ok(()),
                &mut std::io::stdout(),
                options,
            )
            .unwrap();
        }

        visit_error_paths(
            self.config
                .work_limit
                .unwrap_or_else(|| Config::default().work_limit.unwrap()),
            cx,
            fn_kind,
            mir,
            |path, contributing_calls, span| {
                // smoelius: The path is from a return to the start block.
                for (i, &index) in path.iter().enumerate() {
                    if !contributing_calls.contains(index)
                        && let Some((func, func_span)) = is_call_with_mut_ref(cx, mir, &path[i..])
                    {
                        span_lint_and_then(
                            cx,
                            NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
                            func_span,
                            format!(
                                "call to `{func:?}` with mutable reference before error return"
                            ),
                            error_note(span),
                        );
                    }

                    let basic_block = &mir.basic_blocks[index];
                    for statement in basic_block.statements.iter().rev() {
                        if let Some(assign_span) = is_deref_assign(statement) {
                            span_lint_and_then(
                                cx,
                                NON_LOCAL_EFFECT_BEFORE_ERROR_RETURN,
                                assign_span,
                                "assignment to dereference before error return",
                                error_note(span),
                            );
                        }
                    }
                }
            },
        );
    }
}

fn in_async_function(tcx: ty::TyCtxt<'_>, hir_id: rustc_hir::HirId) -> bool {
    std::iter::once((hir_id, tcx.hir_node(hir_id)))
        .chain(tcx.hir().parent_iter(hir_id))
        .any(|(_, node)| {
            node.fn_kind()
                .is_some_and(|fn_kind| fn_kind.asyncness().is_async())
        })
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
) -> Option<(&'tcx Operand<'tcx>, Span)> {
    let index = path[0];
    let basic_block = &mir[index];
    let terminator = basic_block.terminator();
    if let TerminatorKind::Call {
            func,
            args,
            fn_span,
            ..
        } = &terminator.kind
        // smoelius: `deref_mut` generates too much noise.
        && func.const_fn_def().is_none_or(|(def_id, _)| {
            !cx.tcx.is_diagnostic_item(sym::deref_mut_method, def_id)
        })
        && let (locals, constants) = collect_locals_and_constants(cx, mir, path, args.iter().map(|arg| &arg.node))
        && (locals.iter().any(|local| is_mut_ref_arg(mir, local))
            || constants.iter().any(|constant| is_const_ref(constant)))
    {
        Some((func, *fn_span))
    } else {
        None
    }
}

// smoelius: Roughly, a "followed" local is assumed to refer to mutable memory. Locals are followed
// "narrowly" or "widely," and functions are "narrowing," "width preserving," or "widening." If a
// function outputs to a followed local, then the function's inputs are followed according to the
// next table:
//
//                    +--------------------+-------------------+---------------------------+
//                    | narrowing function | width-preserving function | widening function |
//   +----------------+--------------------+---------------------------+-------------------+
//   | local followed |  mut ref inputs    |        all inputs         |                   |
//   | narrowly       | followed narrowly  |     followed narrowly     |                   |
//   +----------------+------------------------------------------------+                   |
//   | local followed |                                                     all inputs     |
//   | widely         |                                                   followed widely  |
//   +----------------+--------------------------------------------------------------------+
//
// Locals are followed narrowly by default, and most functions are narrowing.
//
// Intuitively, a widening function casts an immutable reference to a mutable one, thereby requiring
// that the set of followed locals be "widened."
//
// Width-preserving functions are a bit of a hack. They essentially provide a way of delaying the
// determination of whether a followed local is output to by a narrowing or widening function. At
// present, I am not sure what the "right" solution would be---perhaps another pass, preceding the
// current one, to identify all of the widening functions.

const WIDTH_PRESERVING: &[&[&str]] = &[&["core", "result", "Result", "unwrap"]];

const WIDENING: &[&[&str]] = &[&["std", "sync", "poison", "mutex", "Mutex", "lock"]];

fn collect_locals_and_constants<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    path: &[BasicBlock],
    args: impl Iterator<Item = &'tcx Operand<'tcx>>,
) -> (DenseBitSet<Local>, Vec<&'tcx ConstOperand<'tcx>>) {
    let mut locals_narrowly = DenseBitSet::new_empty(mir.local_decls.len());
    let mut locals_widely = DenseBitSet::new_empty(mir.local_decls.len());
    let mut constants = Vec::new();

    for arg in args {
        if let Some(arg_place) = mut_ref_operand_place(cx, mir, arg) {
            locals_narrowly.insert(arg_place.local);
        }
    }

    if locals_narrowly.is_empty() {
        return (locals_narrowly, constants);
    }

    for (i, &index) in path.iter().enumerate() {
        let basic_block = &mir[index];

        if i != 0 {
            let terminator = basic_block.terminator();
            if let TerminatorKind::Call {
                func,
                destination,
                args,
                ..
            } = &terminator.kind
                && let followed_narrowly = locals_narrowly.remove(destination.local)
                && let followed_widely = locals_widely.remove(destination.local)
                && (followed_narrowly || followed_widely)
            {
                let width_preserving = func.const_fn_def().is_some_and(|(def_id, _)| {
                    WIDTH_PRESERVING
                        .iter()
                        .any(|path| match_def_path(cx, def_id, path))
                });
                let widening = func.const_fn_def().is_some_and(|(def_id, _)| {
                    WIDENING.iter().any(|path| match_def_path(cx, def_id, path))
                });
                for arg in args {
                    let mut_ref_operand_place = mut_ref_operand_place(cx, mir, &arg.node);
                    let arg_place = arg.node.place();
                    if followed_narrowly
                        && !widening
                        && let Some(arg_place) = mut_ref_operand_place.or(if width_preserving {
                            arg_place
                        } else {
                            None
                        })
                    {
                        locals_narrowly.insert(arg_place.local);
                    }
                    if (followed_widely || widening)
                        && let Some(arg_place) = arg_place
                    {
                        locals_widely.insert(arg_place.local);
                    }
                }
            }
        }

        for (statement_index, statement) in basic_block.statements.iter().enumerate().rev() {
            if let StatementKind::Assign(box (assign_place, rvalue)) = &statement.kind
                && let followed_narrowly = locals_narrowly.remove(assign_place.local)
                && let followed_widely = locals_widely.remove(assign_place.local)
                && (followed_narrowly || followed_widely)
            {
                if let Rvalue::Use(Operand::Constant(constant)) = rvalue {
                    constants.push(constant);
                } else if let [rvalue_place, ..] = rvalue_places(
                    rvalue,
                    Location {
                        block: index,
                        statement_index,
                    },
                )
                .as_slice()
                {
                    if followed_narrowly {
                        locals_narrowly.insert(rvalue_place.local);
                    }
                    if followed_widely {
                        locals_widely.insert(rvalue_place.local);
                    }
                }
            }
        }
    }

    locals_narrowly.union(&locals_widely);

    (locals_narrowly, constants)
}

#[rustfmt::skip]
// smoelius: From: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.Body.html#structfield.local_decls
// The first local is the return value pointer, followed by `arg_count` locals for the function arguments, ...
//                                                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
fn is_mut_ref_arg<'tcx>(mir: &'tcx Body<'tcx>, local: Local) -> bool {
    (1..=mir.arg_count).contains(&local.into()) && is_mut_ref(mir.local_decls[local].ty)
}

fn is_const_ref(constant: &ConstOperand<'_>) -> bool {
    constant.ty().is_ref()
}

fn mut_ref_operand_place<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &'tcx Body<'tcx>,
    operand: &Operand<'tcx>,
) -> Option<Place<'tcx>> {
    if let Some(operand_place) = operand.place()
        && is_mut_ref(operand_place.ty(&mir.local_decls, cx.tcx).ty)
    {
        Some(operand_place)
    } else {
        None
    }
}

fn is_mut_ref(ty: ty::Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut))
}

fn is_deref_assign(statement: &Statement) -> Option<Span> {
    if let StatementKind::Assign(box (Place { projection, .. }, _)) = &statement.kind
        && projection.iter().any(|elem| elem == ProjectionElem::Deref)
    {
        Some(statement.source_info.span)
    } else {
        None
    }
}

fn error_note(span: Option<Span>) -> impl FnOnce(&mut Diag<'_, ()>) {
    move |diag| {
        if let Some(span) = span {
            diag.span_note(span, "error is determined here");
        }
    }
}

#[must_use]
fn enabled(opt: &str) -> bool {
    let key = env!("CARGO_PKG_NAME").to_uppercase() + "_" + opt;
    std::env::var(key).is_ok_and(|value| value != "0")
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}

#[test]
fn ui_public_only() {
    dylint_testing::ui::Test::example(env!("CARGO_PKG_NAME"), "ui_public_only")
        .dylint_toml("non_local_effect_before_error_return.public_only = false")
        .run();
}

#[test]
fn ui_main_rs_equal() {
    let ui_main_rs = std::fs::read_to_string("ui/main.rs").unwrap();
    let ui_public_only_main_rs = std::fs::read_to_string("ui_public_only/main.rs").unwrap();
    assert_eq!(ui_main_rs, ui_public_only_main_rs);
}
