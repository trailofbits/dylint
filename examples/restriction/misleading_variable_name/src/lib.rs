#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_help,
    ty::{implements_trait, is_type_diagnostic_item},
};
use heck::ToSnakeCase;
use if_chain::if_chain;
use rustc_hir::{
    def::{DefKind, Res},
    def_id::DefId,
    Expr, ExprKind, LangItem, Local, MatchSource, Pat, PatKind, QPath, Stmt, StmtKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::{sym, symbol::Symbol};
use std::collections::BTreeMap;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for variables satisfying the following three conditions:
    /// - The variable is initialized with the result of a function call.
    /// - The variable's name matches the name of a type defined within the module in which the
    ///   function is defined.
    /// - The variable's type is not the matched type.
    ///
    /// ### Why is this bad?
    /// A reader could mistakenly believe the variable has a type other than the one it actually
    /// has.
    ///
    /// ### Example
    /// ```rust,no_run
    /// # use std::{fs::read_to_string, path::Path};
    /// # let path = Path::new("x");
    /// let file = read_to_string(path).unwrap();
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// # use std::{fs::read_to_string, path::Path};
    /// # let path = Path::new("x");
    /// let contents = read_to_string(path).unwrap();
    /// ```
    pub MISLEADING_VARIABLE_NAME,
    Warn,
    "variables whose names suggest they have types other than the ones they have"
}

impl<'tcx> LateLintPass<'tcx> for MisleadingVariableName {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if_chain! {
            if let StmtKind::Local(Local {
                pat:
                    Pat {
                        kind: PatKind::Binding(_, _, ident, _),
                        ..
                    },
                init: Some(init),
                ..
            }) = stmt.kind;
            let expr = peel_try_unwrap_and_similar(cx, init);
            if let Some(callee_def_id) = callee_def_id(cx, expr);
            let module_def_id = parent_module(cx.tcx, callee_def_id);
            // smoelius: Don't flag functions/types defined in the same module as the call.
            if module_def_id != cx.tcx.parent_module(stmt.hir_id).to_def_id();
            let child_types = module_public_child_types(cx.tcx, module_def_id);
            if let Some((child_ty_name, child_ty)) = child_types.get(ident.name.as_str());
            let init_ty = erase_substs(
                cx.tcx,
                peel_refs_and_rcs(cx, module_def_id, cx.typeck_results().expr_ty(init)),
            );
            if init_ty != *child_ty;
            then {
                let help_msg = child_types
                    .iter()
                    .find_map(|(name, &(_, child_ty))| {
                        if init_ty == child_ty {
                            Some(format!("use `{name}` or something similar"))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        if child_types.len() == 1 {
                            format!(
                                "use a name other than `{}`",
                                child_types.keys().next().unwrap()
                            )
                        } else {
                            let mut names = child_types
                                .keys()
                                .map(|s| format!("`{s}`"))
                                .collect::<Vec<_>>();
                            let last = names.pop().unwrap();
                            format!(
                                "use a name that is not {}{} or {}",
                                names.join(", "),
                                if names.len() >= 2 { "," } else { "" },
                                last
                            )
                        }
                    });
                span_lint_and_help(
                    cx,
                    MISLEADING_VARIABLE_NAME,
                    ident.span,
                    &format!(
                        "`{}` exports a type `{}`, which is not the type of `{}`",
                        cx.tcx.def_path_str(module_def_id),
                        child_ty_name,
                        ident.name
                    ),
                    None,
                    &help_msg,
                )
            }
        }
    }
}

fn peel_refs_and_rcs<'tcx>(
    cx: &LateContext<'tcx>,
    module_def_id: DefId,
    mut ty: ty::Ty<'tcx>,
) -> ty::Ty<'tcx> {
    loop {
        match ty.kind() {
            ty::Ref(_, referent_ty, _) => {
                ty = *referent_ty;
            }
            // smoelius: If the initializer originates from the same module as `Arc` or `Rc`, don't
            // peel them.
            ty::Adt(adt_def, substs)
                if module_def_id != parent_module(cx.tcx, adt_def.did())
                    && (is_type_diagnostic_item(cx, ty, sym::Arc)
                        || is_type_diagnostic_item(cx, ty, sym::Rc)) =>
            {
                ty = substs[0].expect_ty();
            }
            _ => {
                break;
            }
        }
    }

    ty
}

fn peel_try_unwrap_and_similar<'tcx>(
    cx: &LateContext<'_>,
    mut expr: &'tcx Expr<'tcx>,
) -> &'tcx Expr<'tcx> {
    loop {
        match expr.kind {
            ExprKind::Match(scrutinee, _, MatchSource::TryDesugar) => {
                if let ExprKind::Call(
                    Expr {
                        kind: ExprKind::Path(QPath::LangItem(LangItem::TryTraitBranch, _, _)),
                        ..
                    },
                    [arg],
                ) = scrutinee.kind
                {
                    expr = arg;
                } else {
                    break;
                }
            }
            ExprKind::MethodCall(method, receiver, args, _)
                if (method.ident.name == sym::unwrap && args.is_empty())
                    || (is_try_implementor(cx, expr) && is_try_implementor(cx, receiver)) =>
            {
                expr = receiver;
            }
            _ => {
                break;
            }
        }
    }
    expr
}

fn is_try_implementor(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let expr_ty = cx.typeck_results().expr_ty(expr);
    if_chain! {
        if let Some(try_trait_def_id) = cx.tcx.lang_items().try_trait();
        if implements_trait(cx, expr_ty, try_trait_def_id, &[]);
        then {
            true
        } else {
            false
        }
    }
}

fn callee_def_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<DefId> {
    match expr.kind {
        ExprKind::Call(callee, _) => {
            let callee_ty = cx.typeck_results().expr_ty(callee);
            if let ty::FnDef(callee_def_id, _) = callee_ty.kind() {
                Some(*callee_def_id)
            } else {
                None
            }
        }
        ExprKind::MethodCall(..) => cx.typeck_results().type_dependent_def_id(expr.hir_id),
        _ => None,
    }
}

fn parent_module(tcx: ty::TyCtxt<'_>, mut def_id: DefId) -> DefId {
    while tcx.def_kind(def_id) != DefKind::Mod {
        def_id = tcx.parent(def_id);
    }
    def_id
}

fn module_public_child_types(
    tcx: ty::TyCtxt<'_>,
    module_def_id: DefId,
) -> BTreeMap<String, (Symbol, ty::Ty<'_>)> {
    let mut child_types = BTreeMap::new();
    for (child_name, child_def_id) in module_public_children(tcx, module_def_id) {
        if matches!(
            tcx.def_kind(child_def_id),
            DefKind::Struct | DefKind::Union | DefKind::Enum | DefKind::TyAlias,
        ) {
            child_types.insert(
                child_name.as_str().to_snake_case(),
                (
                    child_name,
                    erase_substs(tcx, tcx.type_of(child_def_id).skip_binder().peel_refs()),
                ),
            );
        }
    }
    child_types
}

fn module_public_children(tcx: ty::TyCtxt<'_>, module_def_id: DefId) -> Vec<(Symbol, DefId)> {
    if let Some(module_local_def_id) = module_def_id.as_local() {
        tcx.hir()
            .module_items(module_local_def_id)
            .filter_map(|item_id| {
                let child_def_id = item_id.owner_id.to_def_id();
                if tcx.visibility(child_def_id).is_public() {
                    let item = tcx.hir().item(item_id);
                    Some((item.ident.name, child_def_id))
                } else {
                    None
                }
            })
            .collect()
    } else {
        tcx.module_children(module_def_id)
            .iter()
            .filter_map(|child| {
                if_chain! {
                    if child.vis == ty::Visibility::Public;
                    if let Res::Def(_, child_def_id) = child.res;
                    then {
                        Some((child.ident.name, child_def_id))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }
}

// smoelius: `erase_substs` is incomplete.
fn erase_substs<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> ty::Ty<'tcx> {
    match ty.kind() {
        ty::Adt(adt_def, _) => tcx.mk_adt(*adt_def, ty::List::empty()),
        _ => ty,
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
