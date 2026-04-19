#![feature(rustc_private)]
#![feature(box_patterns)]
#![warn(unused_extern_crates)]

extern crate rustc_abi;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_then;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    def_id::{DefId, LocalDefId},
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{BasicBlock, TerminatorKind, pretty::MirWriter};
use rustc_span::Span;
use serde::Deserialize;
use std::cell::RefCell;

mod non_local_effect;
use non_local_effect::{
    NonLocalEffect, NonLocalEffectKind, has_non_local_effect_before_error_return,
};

mod rvalue_places;

mod unhandled_error;

mod visit_error_paths;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for calls whose errors may be unhandled and whose callees perform non-local effects
    /// (e.g., assignments to mutable references) before returning an error.
    ///
    /// ### Why is this bad?
    ///
    /// Functions that make changes to the program state before returning an error are difficult
    /// to reason about: generally speaking, if a function returns an error, it should be as
    /// though the function was never called. Failing to handle an error returned by such a
    /// function compounds the problem, because the caller silently leaves the program in a
    /// partially-modified state.
    ///
    /// This lint is interprocedural: it identifies functions that may perform non-local effects
    /// before returning an error, then flags call sites that do not handle the errors returned
    /// by those functions.
    ///
    /// ### Known problems
    ///
    /// - The search strategy for detecting non-local effects is exponential in the number of
    ///   blocks in a function body. To help deal with complex bodies, the lint includes a "work
    ///   limit" (see "Configuration" below).
    /// - Errors in loops are not handled properly.
    /// - Interprocedural tracking is limited to functions whose MIR is available (i.e., functions
    ///   defined in the current crate).
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
    ///
    /// fn caller(account: &mut Account) {
    ///     let _ = account.withdraw(100);
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
    ///
    /// fn caller(account: &mut Account) -> Result<(), InsufficientBalance> {
    ///     account.withdraw(100)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ### Configuration
    ///
    /// - `work_limit: u64` (default 500000): When exploring a function body for non-local
    ///   effects, the maximum number of times the search path is extended. Setting this to a
    ///   higher number allows more bodies to be explored exhaustively, but at the expense of
    ///   greater runtime.
    pub NON_LOCAL_EFFECT_BEFORE_UNHANDLED_ERROR,
    Warn,
    "unhandled errors from functions with non-local effects before error return",
    NonLocalEffectBeforeUnhandledError::new()
}

#[derive(Deserialize)]
struct Config {
    work_limit: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_limit: Some(500_000),
        }
    }
}

struct NonLocalEffectBeforeUnhandledError {
    config: Config,
    non_local_effects: RefCell<FxHashMap<DefId, Option<NonLocalEffect>>>,
}

impl NonLocalEffectBeforeUnhandledError {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
            non_local_effects: RefCell::new(FxHashMap::default()),
        }
    }

    fn work_limit(&self) -> u64 {
        self.config
            .work_limit
            .unwrap_or_else(|| Config::default().work_limit.unwrap())
    }

    fn get_non_local_effect(&self, cx: &LateContext<'_>, def_id: DefId) -> Option<NonLocalEffect> {
        if let Some(cached) = self.non_local_effects.borrow().get(&def_id) {
            return cached.clone();
        }
        let result = has_non_local_effect_before_error_return(cx, def_id, self.work_limit());
        self.non_local_effects
            .borrow_mut()
            .insert(def_id, result.clone());
        result
    }
}

impl<'tcx> LateLintPass<'tcx> for NonLocalEffectBeforeUnhandledError {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _fn_kind: FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'_>,
        _body: &'tcx rustc_hir::Body<'_>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }

        let def_id = local_def_id.to_def_id();

        if !cx.tcx.is_mir_available(def_id) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id);

        if enabled("DEBUG_MIR") {
            let writer = MirWriter::new(cx.tcx);
            writer.write_mir_fn(mir, &mut std::io::stdout()).unwrap();
        }

        // Collect call sites whose callees have non-local effects. Skip the dataflow analysis
        // entirely if none are found.
        let mut non_local_calls: Vec<(BasicBlock, DefId, Span, NonLocalEffect)> = Vec::new();
        for (block, basic_block) in mir.basic_blocks.iter_enumerated() {
            let terminator = basic_block.terminator();
            let TerminatorKind::Call {
                func,
                destination,
                fn_span,
                target: Some(_),
                ..
            } = &terminator.kind
            else {
                continue;
            };

            if !destination.projection.is_empty() {
                continue;
            }

            let Some((callee_def_id, _)) = func.const_fn_def() else {
                continue;
            };

            if callee_def_id == def_id {
                // Skip direct self-recursion.
                continue;
            }

            let Some(info) = self.get_non_local_effect(cx, callee_def_id) else {
                continue;
            };

            non_local_calls.push((block, callee_def_id, *fn_span, info));
        }

        if non_local_calls.is_empty() {
            return;
        }

        let state_at_end = unhandled_error::analyze(cx, mir);

        // Deduplicate warnings by callee def_id so each callee is reported at most once
        // per enclosing function, keeping the earliest (by span) call site as the primary.
        let mut reported: FxHashMap<DefId, (Span, NonLocalEffect)> = FxHashMap::default();

        for (block, callee_def_id, fn_span, info) in non_local_calls {
            let destination_local = match &mir[block].terminator().kind {
                TerminatorKind::Call { destination, .. } => destination.local,
                _ => continue,
            };

            if !state_at_end[block].contains(destination_local) {
                continue;
            }

            let existing = reported
                .entry(callee_def_id)
                .or_insert_with(|| (fn_span, info.clone()));
            if fn_span.lo() < existing.0.lo() {
                *existing = (fn_span, info);
            }
        }

        for (callee_def_id, (fn_span, info)) in reported {
            let callee = cx.tcx.def_path_str(callee_def_id);
            emit_warning(cx, &callee, fn_span, &info);
        }
    }
}

fn emit_warning(cx: &LateContext<'_>, callee: &str, fn_span: Span, info: &NonLocalEffect) {
    span_lint_and_then(
        cx,
        NON_LOCAL_EFFECT_BEFORE_UNHANDLED_ERROR,
        fn_span,
        format!(
            "unhandled call to `{callee}`, which performs a non-local effect before returning \
             an error"
        ),
        |diag| {
            match &info.kind {
                NonLocalEffectKind::Call {
                    callee: inner_callee,
                    span,
                } => {
                    diag.span_note(*span, format!("non-local effect: call to `{inner_callee}`"));
                }
                NonLocalEffectKind::DerefAssign { span } => {
                    diag.span_note(*span, "non-local effect: assignment to dereference");
                }
            }
            if let Some(error_span) = info.error_span {
                diag.span_note(error_span, "error is determined here");
            }
        },
    );
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
