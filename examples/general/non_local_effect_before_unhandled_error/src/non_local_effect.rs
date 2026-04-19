use clippy_utils::{
    paths::{PathLookup, PathNS},
    type_path,
};
use dylint_internal::match_def_path;
use rustc_hir::HirId;
use rustc_index::bit_set::DenseBitSet;
use rustc_lint::LateContext;
use rustc_middle::{
    mir::{
        BasicBlock, Body, ConstOperand, Local, Location, Mutability, Operand, Place,
        ProjectionElem, Rvalue, Statement, StatementKind, TerminatorKind,
    },
    ty,
};
use rustc_span::{Span, sym};

use crate::rvalue_places::rvalue_places;

pub fn in_async_function(tcx: ty::TyCtxt<'_>, hir_id: HirId) -> bool {
    std::iter::once((hir_id, tcx.hir_node(hir_id)))
        .chain(tcx.hir_parent_iter(hir_id))
        .any(|(_, node)| {
            node.fn_kind()
                .is_some_and(|fn_kind| fn_kind.asyncness().is_async())
        })
}

static CORE_FMT_ERROR: PathLookup = type_path!(core::fmt::Error);

pub fn is_lintable_result(cx: &LateContext<'_>, ty: ty::Ty) -> bool {
    if let ty::Adt(adt, substs) = ty.kind() {
        if !cx.tcx.is_diagnostic_item(sym::Result, adt.did()) {
            return false;
        }

        // Don't lint if the error type is core::fmt::Error
        if let Some(error_ty) = substs.get(1)
            && let ty::Adt(error_adt, _) = error_ty.expect_ty().kind()
            && CORE_FMT_ERROR.matches(cx, error_adt.did())
        {
            return false;
        }

        true
    } else {
        false
    }
}

pub fn is_call_with_mut_ref<'tcx>(
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
    args: impl IntoIterator<Item = &'tcx Operand<'tcx>>,
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

pub fn is_deref_assign(statement: &Statement) -> Option<Span> {
    if let StatementKind::Assign(box (Place { projection, .. }, _)) = &statement.kind
        && projection.iter().any(|elem| elem == ProjectionElem::Deref)
    {
        Some(statement.source_info.span)
    } else {
        None
    }
}
