#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    get_parent_expr,
    ty::{is_copy, peel_mid_ty_refs},
};
use dylint_internal::env::enabled;
use if_chain::if_chain;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{
    def_id::LocalDefId,
    intravisit::{walk_generic_param, walk_lifetime, Visitor},
    Expr, ExprKind, GenericParam, GenericParamKind, HirId, Item, ItemKind, Lifetime, LifetimeName,
    MutTy, Mutability, ParamName, TyKind, VariantData,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{symbol::Ident, Span};
use std::collections::HashSet;

dylint_linting::impl_late_lint! {
    /// **What it does:** Checks for fields that are references used only to read one copyable
    /// subfield, and whose lifetimes are not used elsewhere.
    ///
    /// **Why is this bad?** Storing the reference instead of a copy of the subfield adds an
    /// unnecessary lifetime parameter to the struct. It also creates an unnecessary pointer
    /// dereference at runtime.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_hir;
    /// # extern crate rustc_lint;
    /// # extern crate rustc_middle;
    /// # use rustc_hir::intravisit::Visitor;
    /// # use rustc_lint::LateContext;
    /// struct V<'cx, 'tcx> {
    ///     cx: &'cx LateContext<'tcx>,
    /// }
    ///
    /// impl<'cx, 'tcx> Visitor<'tcx> for V<'cx, 'tcx>
    /// {
    ///     type Map = rustc_middle::hir::map::Map<'tcx>;
    ///     type NestedFilter = rustc_middle::hir::nested_filter::All;
    ///
    ///     fn nested_visit_map(&mut self) -> Self::Map {
    ///         self.cx.tcx.hir()
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # extern crate rustc_hir;
    /// # extern crate rustc_lint;
    /// # extern crate rustc_middle;
    /// # use rustc_hir::intravisit::Visitor;
    /// # use rustc_middle::ty::TyCtxt;
    /// struct V<'tcx> {
    ///     tcx: TyCtxt<'tcx>,
    /// }
    ///
    /// impl<'tcx> Visitor<'tcx> for V<'tcx>
    /// {
    ///     type Map = rustc_middle::hir::map::Map<'tcx>;
    ///     type NestedFilter = rustc_middle::hir::nested_filter::All;
    ///
    ///     fn nested_visit_map(&mut self) -> Self::Map {
    ///         self.tcx.hir()
    ///     }
    /// }
    /// ```
    ///
    /// **Options:**
    /// `REDUNDANT_REFERENCE_NO_LIFETIME_CHECK`: Setting this environment variable to anything other
    /// than `0` disables the check that the lifetime use is unique. That is, the lint becomes a
    /// check for: fields that are references used only to read one copyable subfield.
    pub REDUNDANT_REFERENCE,
    Warn,
    "reference fields used only to read one copyable subfield",
    RedundantReference::default()
}

const REDUNDANT_REFERENCE_NO_LIFETIME_CHECK: &str = "REDUNDANT_REFERENCE_NO_LIFETIME_CHECK";

#[derive(Default)]
struct FieldUse {
    subfield_accesses: FxHashMap<Ident, (String, FxHashSet<Span>)>,
    other_use: bool,
}

#[derive(Default)]
struct RedundantReference {
    field_uses: FxHashMap<(LocalDefId, Ident), FieldUse>,
}

impl<'tcx> LateLintPass<'tcx> for RedundantReference {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if_chain! {
            if let ExprKind::Field(operand, field) = expr.kind;
            let (operand_ty, _) = peel_mid_ty_refs(cx.typeck_results().expr_ty(operand));
            if let ty::Adt(adt_def, _) = operand_ty.kind();
            if let Some(local_def_id) = adt_def.did().as_local();
            if let Some(parent) = get_parent_expr(cx, expr);
            // smoelius: `typeck_results` cannot be called outside of the body. So the subfield's
            // type is checked here.
            let parent_ty = cx.typeck_results().expr_ty(parent);
            if is_copy(cx, parent_ty);
            then {
                let field_use = self
                    .field_uses
                    .entry((local_def_id, field))
                    .or_insert_with(Default::default);
                if let ExprKind::Field(_, subfield) = parent.kind {
                    let subfield_access = field_use
                        .subfield_accesses
                        .entry(subfield)
                        .or_insert((parent_ty.to_string(), HashSet::default()));
                    subfield_access
                        .1
                        .insert(subfield.span.with_lo(operand.span.hi()));
                } else {
                    field_use.other_use = true;
                }
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
            let item = cx.tcx.hir().expect_item(*local_def_id);
            if_chain! {
                if let ItemKind::Struct(VariantData::Struct(field_defs, _), _) = &item.kind;
                if let Some(field_def) = field_defs
                    .iter()
                    .find(|field_def| field_def.ident == *field);
                let field_def_local_def_id = cx.tcx.hir().local_def_id(field_def.hir_id);
                if !cx.tcx.visibility(*local_def_id).is_public()
                    || !cx.tcx.visibility(field_def_local_def_id).is_public();
                if let TyKind::Rptr(
                    lifetime,
                    MutTy {
                        ty: _,
                        mutbl: Mutability::Not,
                    },
                ) = field_def.ty.kind;
                if let LifetimeName::Param(_, ParamName::Plain(ident)) = lifetime.name;
                if enabled(REDUNDANT_REFERENCE_NO_LIFETIME_CHECK) || {
                    let lifetime_uses = lifetime_uses(ident, item);
                    lifetime_uses.len() == 1 && {
                        assert_eq!(
                            lifetime_uses.iter().copied().next().unwrap(),
                            lifetime.hir_id
                        );
                        true
                    }
                };
                if subfield_accesses.keys().len() == 1;
                if !other_use;
                then {
                    let (lifetime_msg, lifetime_help) =
                        if enabled(REDUNDANT_REFERENCE_NO_LIFETIME_CHECK) {
                            (String::new(), " instead".to_owned())
                        } else {
                            (
                                format!(
                                    " is the only field of `{}` that uses lifetime `{}`, and",
                                    item.ident, lifetime
                                ),
                                format!(" to eliminate the need for `{}`", lifetime),
                            )
                        };
                    let (subfield, (subfield_ty, access_spans)) =
                        subfield_accesses.iter().next().unwrap();
                    cx.struct_span_lint(REDUNDANT_REFERENCE, field_def.span, |diag| {
                        let mut diag = diag.build(&format!(
                            "`.{}`{} is used only to read `.{}.{}`, \
                            whose type `{}` implements `Copy`",
                            field, lifetime_msg, field, subfield, subfield_ty
                        ));
                        for access_span in access_spans {
                            diag.span_note(*access_span, &"read here".to_owned());
                        }
                        diag.help(&format!(
                            "consider storing a copy of `.{}.{}`{}",
                            field, subfield, lifetime_help
                        ));
                        diag.emit();
                    });
                }
            }
        }
    }
}

fn lifetime_uses(ident: Ident, item: &Item<'_>) -> FxHashSet<HirId> {
    let mut visitor = LifetimeUses {
        ident,
        uses: Default::default(),
    };
    visitor.visit_item(item);
    visitor.uses
}

struct LifetimeUses {
    ident: Ident,
    uses: FxHashSet<HirId>,
}

impl<'tcx> Visitor<'tcx> for LifetimeUses {
    fn visit_lifetime(&mut self, lifetime: &'tcx Lifetime) {
        if_chain! {
            if let LifetimeName::Param(_, ParamName::Plain(ident)) = lifetime.name;
            if ident == self.ident;
            then {
                self.uses.insert(lifetime.hir_id);
            }
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

#[cfg_attr(
    dylint_lib = "non_thread_safe_call_in_test",
    allow(non_thread_safe_call_in_test)
)]
#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );

    // smoelius: For some reason, the diagnostic messages are printed in a different order on Linux
    // than on Mac and Windows.
    // smoelius: However, the current workaround should allow the tests to succeed on all platforms.
    // if cfg!(not(target_os = "linux")) {
    //     return;
    // }

    // smoelius: There doesn't seem to be a way to set environment variables using `compiletest`'s
    // [`Config`](https://docs.rs/compiletest_rs/0.7.1/compiletest_rs/common/struct.Config.html)
    // struct. For comparison, where Clippy uses `compiletest`, it sets environment variables
    // directly (see: https://github.com/rust-lang/rust-clippy/blob/master/tests/compile-test.rs).
    //   Of course, even if `compiletest` had such support, it would need to be incorporated into
    // `dylint_testing`.
    std::env::set_var(REDUNDANT_REFERENCE_NO_LIFETIME_CHECK, "1");

    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_no_lifetime_check"),
    );
}

#[test]
fn ui_main_rs_starts_with() {
    let ui_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui")
            .join("main.rs"),
    )
    .unwrap();
    let ui_no_lifetime_check_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui_no_lifetime_check")
            .join("main.rs"),
    )
    .unwrap();
    assert!(ui_main_rs.starts_with(&ui_no_lifetime_check_main_rs));
}
