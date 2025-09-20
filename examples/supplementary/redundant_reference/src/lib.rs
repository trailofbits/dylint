#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_hir_and_then,
    get_parent_expr,
    ty::{is_copy, peel_and_count_ty_refs},
};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{
    Expr, ExprKind, GenericParam, GenericParamKind, HirId, Item, ItemKind, Lifetime, LifetimeKind,
    MutTy, Mutability, TyKind, VariantData,
    def_id::LocalDefId,
    intravisit::{Visitor, walk_generic_param, walk_lifetime},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::{Span, symbol::Ident};
use serde::Deserialize;
use std::collections::HashSet;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for fields that are references used only to read one copyable subfield, and whose
    /// lifetimes are not used elsewhere.
    ///
    /// ### Why is this bad?
    ///
    /// Storing the reference instead of a copy of the subfield adds an unnecessary lifetime
    /// parameter to the struct. It also creates an unnecessary pointer dereference at runtime.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_driver;
    /// # extern crate rustc_hir;
    /// # extern crate rustc_lint;
    /// # extern crate rustc_middle;
    /// # use rustc_hir::intravisit::Visitor;
    /// # use rustc_lint::LateContext;
    /// struct V<'cx, 'tcx> {
    ///     cx: &'cx LateContext<'tcx>,
    /// }
    ///
    /// impl<'cx, 'tcx> Visitor<'tcx> for V<'cx, 'tcx> {
    ///     type MaybeTyCtxt = rustc_middle::ty::TyCtxt<'tcx>;
    ///     type NestedFilter = rustc_middle::hir::nested_filter::All;
    ///
    ///     fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
    ///         self.cx.tcx
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_driver;
    /// # extern crate rustc_hir;
    /// # extern crate rustc_lint;
    /// # extern crate rustc_middle;
    /// # use rustc_hir::intravisit::Visitor;
    /// # use rustc_middle::ty::TyCtxt;
    /// struct V<'tcx> {
    ///     tcx: TyCtxt<'tcx>,
    /// }
    ///
    /// impl<'tcx> Visitor<'tcx> for V<'tcx> {
    ///     type MaybeTyCtxt = rustc_middle::ty::TyCtxt<'tcx>;
    ///     type NestedFilter = rustc_middle::hir::nested_filter::All;
    ///
    ///     fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
    ///         self.tcx
    ///     }
    /// }
    /// ```
    ///
    /// ### Configuration
    ///
    /// - `lifetime_check: bool` (default `true`): Setting this to `false` disables the check that
    ///   the lifetime use is unique. That is, the lint becomes a check for: fields that are
    ///   references used only to read one copyable subfield.
    pub REDUNDANT_REFERENCE,
    Warn,
    "reference fields used only to read one copyable subfield",
    RedundantReference::new()
}

#[derive(Deserialize)]
struct Config {
    lifetime_check: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lifetime_check: true,
        }
    }
}

#[derive(Default)]
struct FieldUse {
    subfield_accesses: FxHashMap<Ident, (String, FxHashSet<Span>)>,
    other_use: bool,
}

struct RedundantReference {
    config: Config,
    field_uses: FxHashMap<(LocalDefId, Ident), FieldUse>,
}

impl RedundantReference {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
            field_uses: FxHashMap::default(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for RedundantReference {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Field(operand, field) = expr.kind
            && let (operand_ty, _, _) = peel_and_count_ty_refs(cx.typeck_results().expr_ty(operand))
            && let ty::Adt(adt_def, _) = operand_ty.kind()
            && let Some(local_def_id) = adt_def.did().as_local()
            && let Some(parent) = get_parent_expr(cx, expr)
            // smoelius: `typeck_results` cannot be called outside of the body. So the subfield's
            // type is checked here.
            && let parent_ty = cx.typeck_results().expr_ty(parent)
            && is_copy(cx, parent_ty)
        {
            let field_use = self.field_uses.entry((local_def_id, field)).or_default();
            if let ExprKind::Field(_, subfield) = parent.kind {
                let subfield_access = field_use
                    .subfield_accesses
                    .entry(subfield)
                    .or_insert_with(|| (parent_ty.to_string(), HashSet::default()));
                subfield_access
                    .1
                    .insert(subfield.span.with_lo(operand.span.hi()));
            } else {
                field_use.other_use = true;
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for (
            (local_def_id, field),
            FieldUse {
                subfield_accesses,
                other_use,
            },
        ) in &self.field_uses
        {
            let item = cx.tcx.hir_expect_item(*local_def_id);
            if let ItemKind::Struct(ident, _generics, VariantData::Struct { fields, .. }) =
                &item.kind
                && let Some(field_def) = fields.iter().find(|field_def| field_def.ident == *field)
                && let field_def_local_def_id = field_def.def_id
                && (!cx.tcx.visibility(*local_def_id).is_public()
                    || !cx.tcx.visibility(field_def_local_def_id).is_public())
                && let TyKind::Ref(
                    lifetime,
                    MutTy {
                        ty: _,
                        mutbl: Mutability::Not,
                    },
                ) = field_def.ty.kind
                && let LifetimeKind::Param(local_def_id) = lifetime.kind
                && (!self.config.lifetime_check || {
                    let lifetime_uses = lifetime_uses(local_def_id, item);
                    lifetime_uses.len() == 1 && {
                        assert_eq!(
                            lifetime_uses.iter().copied().next().unwrap(),
                            lifetime.hir_id
                        );
                        true
                    }
                })
                && subfield_accesses.keys().len() == 1
                && !other_use
            {
                let (lifetime_msg, lifetime_help) = if self.config.lifetime_check {
                    (
                        format!(
                            " is the only field of `{ident}` that uses lifetime `{lifetime}`, and",
                        ),
                        format!(" to eliminate the need for `{lifetime}`"),
                    )
                } else {
                    (String::new(), " instead".to_owned())
                };
                let (subfield, (subfield_ty, access_spans)) =
                    subfield_accesses.iter().next().unwrap();
                span_lint_hir_and_then(
                    cx,
                    REDUNDANT_REFERENCE,
                    item.hir_id(),
                    field_def.span,
                    format!(
                        "`.{field}`{lifetime_msg} is used only to read `.{field}.{subfield}`, \
                        whose type `{subfield_ty}` implements `Copy`"
                    ),
                    |diag| {
                        for access_span in access_spans {
                            diag.span_note(*access_span, "read here");
                        }
                        diag.help(format!(
                            "consider storing a copy of `.{field}.{subfield}`{lifetime_help}"
                        ));
                    },
                );
            }
        }
    }
}

fn lifetime_uses(local_def_id: LocalDefId, item: &Item<'_>) -> FxHashSet<HirId> {
    let mut visitor = LifetimeUses {
        local_def_id,
        uses: FxHashSet::default(),
    };
    visitor.visit_item(item);
    visitor.uses
}

struct LifetimeUses {
    local_def_id: LocalDefId,
    uses: FxHashSet<HirId>,
}

impl<'tcx> Visitor<'tcx> for LifetimeUses {
    fn visit_lifetime(&mut self, lifetime: &'tcx Lifetime) {
        if let LifetimeKind::Param(local_def_id) = lifetime.kind
            && self.local_def_id == local_def_id
        {
            self.uses.insert(lifetime.hir_id);
        }
        walk_lifetime(self, lifetime);
    }

    fn visit_generic_param(&mut self, param: &'tcx GenericParam<'_>) {
        if matches!(param.kind, GenericParamKind::Lifetime { .. }) {
            return;
        }
        walk_generic_param(self, param);
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg_attr(dylint_lib = "supplementary", expect(commented_out_code))]
#[test]
fn ui_no_lifetime_check() {
    // smoelius: For some reason, the diagnostic messages are printed in a different order on Linux
    // than on Mac and Windows.
    // smoelius: However, the current workaround should allow the tests to succeed on all platforms.
    // if cfg!(not(target_os = "linux")) {
    //     return;
    // }

    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_no_lifetime_check")
        .dylint_toml("redundant_reference.lifetime_check = false")
        .run();
}

#[test]
fn ui_main_rs_starts_with() {
    let ui_main_rs = std::fs::read_to_string("ui/main.rs").unwrap();
    let ui_no_lifetime_check_main_rs =
        std::fs::read_to_string("ui_no_lifetime_check/main.rs").unwrap();
    assert!(ui_main_rs.starts_with(&ui_no_lifetime_check_main_rs));
}
