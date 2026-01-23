#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_sugg;
use dylint_internal::{match_def_path, paths};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_index::bit_set::DenseBitSet;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        Body, Local, Location, Mutability, Place, Rvalue, Terminator, TerminatorKind,
        pretty::MirWriter,
        visit::{PlaceContext, Visitor},
    },
    ty::TyCtxt,
};
use rustc_span::{Span, sym};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for calls to [`RefCell::borrow_mut`] that could be calls to [`RefCell::borrow`].
    ///
    /// ### Why is this bad?
    ///
    /// A call to [`RefCell::borrow_mut`] "panics if the value is currently borrowed." Thus, a call
    /// to [`RefCell::borrow_mut`] can panic in situations where a call to [`RefCell::borrow`] would
    /// not.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # let mut x = 0;
    /// # let cell = std::cell::RefCell::new(1);
    /// x = *cell.borrow_mut();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # let mut x = 0;
    /// # let cell = std::cell::RefCell::new(1);
    /// x = *cell.borrow();
    /// ```
    ///
    /// [`RefCell::borrow_mut`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow_mut
    /// [`RefCell::borrow`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow
    pub UNNECESSARY_BORROW_MUT,
    Warn,
    "calls to `RefCell::borrow_mut` that could be `RefCell::borrow`"
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryBorrowMut {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &hir::Body<'tcx>) {
        let local_def_id = cx.tcx.hir_body_owner_def_id(body.id());

        if cx.tcx.hir_body_const_context(local_def_id).is_some() {
            return;
        }

        let mir = cx.tcx.optimized_mir(local_def_id.to_def_id());

        if enabled("DEBUG_MIR") {
            let writer = MirWriter::new(cx.tcx);
            writer.write_mir_fn(mir, &mut std::io::stdout()).unwrap();
        }

        for (local, borrow_mut_span) in collect_borrow_mut_locals(cx, mir) {
            if used_exclusively_for_deref(cx, mir, local) {
                span_lint_and_sugg(
                    cx,
                    UNNECESSARY_BORROW_MUT,
                    borrow_mut_span,
                    "borrowed reference is used only immutably",
                    "use",
                    "borrow()".to_owned(),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}

fn collect_borrow_mut_locals(cx: &LateContext<'_>, mir: &Body) -> Vec<(Local, Span)> {
    mir.basic_blocks
        .iter()
        .filter_map(|basic_block| {
            let terminator = basic_block.terminator();
            if let TerminatorKind::Call {
                func,
                destination,
                fn_span,
                ..
            } = &terminator.kind
                && let Some((def_id, _)) = func.const_fn_def()
                && match_def_path(cx, def_id, &paths::REF_CELL_BORROW_MUT)
                && let Some(local) = destination.as_local()
            {
                Some((local, *fn_span))
            } else {
                None
            }
        })
        .collect()
}

// smoelius: The MIR we want to recognize looks roughly as follows:
//
//         _6 = std::cell::RefCell::<u32>::borrow_mut(move _7) -> bb2;
//         ...
//         _5 = &_6;
//         ...
//         _4 = <std::cell::RefMut<'_, u32> as std::ops::Deref>::deref(move _5) -> [...];

fn used_exclusively_for_deref<'tcx>(
    cx: &LateContext<'tcx>,
    mir: &Body<'tcx>,
    local: Local,
) -> bool {
    let mut visited = DenseBitSet::new_empty(mir.local_decls.len());
    let mut locals = vec![local];

    while let Some(local) = locals.pop() {
        if !visited.insert(local) {
            continue;
        }

        let mut v = V {
            tcx: cx.tcx,
            local,
            outcome: None,
        };

        v.visit_body(mir);

        match v.outcome {
            None => {}
            Some(Outcome::UsedNotForDeref) => {
                return false;
            }
            Some(Outcome::TaintedLocals(tainted)) => locals.extend(tainted),
        }
    }

    true
}

#[derive(Debug)]
enum Outcome {
    UsedNotForDeref,
    TaintedLocals(Vec<Local>),
}

struct V<'tcx> {
    tcx: TyCtxt<'tcx>,
    local: Local,
    outcome: Option<Outcome>,
}

impl<'tcx> Visitor<'tcx> for V<'tcx> {
    fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
        if let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &terminator.kind
        {
            if destination.as_local() == Some(self.local) {
                return;
            }
            if let Some((def_id, _)) = func.const_fn_def()
                && self.tcx.is_diagnostic_item(sym::deref_method, def_id)
                && let [arg] = args.as_ref()
                && let Some(arg_place) = arg.node.place()
                && arg_place.as_local() == Some(self.local)
            {
                return;
            }
        }
        self.super_terminator(terminator, location);
    }

    fn visit_assign(&mut self, place: &Place<'tcx>, rvalue: &Rvalue<'tcx>, location: Location) {
        if place.as_local() == Some(self.local) {
            return;
        }
        if let Rvalue::Ref(_, borrow_kind, referent) = rvalue
            && borrow_kind.to_mutbl_lossy() == Mutability::Not
            && referent.as_local() == Some(self.local)
        {
            match self.outcome {
                None => self.outcome = Some(Outcome::TaintedLocals(vec![place.local])),
                Some(Outcome::UsedNotForDeref) => {}
                Some(Outcome::TaintedLocals(ref mut tainted)) => tainted.push(place.local),
            }
            return;
        }
        self.super_assign(place, rvalue, location);
    }

    // smoelius: `visit_place` is called only if the `if` conditions in `visit_terminator` and
    // `visit_assign` did *not* apply.
    fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _location: Location) {
        if place.local == self.local
            && !context.is_drop()
            && !matches!(context, PlaceContext::NonUse(_))
        {
            self.outcome = Some(Outcome::UsedNotForDeref);
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
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
