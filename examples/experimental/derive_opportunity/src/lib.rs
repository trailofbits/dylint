#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    source::snippet_indent,
    ty::{implements_trait, implements_trait_with_env},
};
use once_cell::sync::OnceCell;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_errors::Applicability;
use rustc_hir::{def_id::DefId, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_middle::{
    traits::Reveal,
    ty::{self, ToPredicate},
};
use rustc_session::{declare_lint, impl_lint_pass, Session};
use rustc_span::{sym, ExpnKind, MacroKind, Symbol};
use serde::Deserialize;
use std::{cell::RefCell, iter};

declare_lint! {
    /// ### What it does
    /// Checks for data structures that could derive additional traits.
    ///
    /// ### Why is this bad?
    /// Not deriving the additional traits could be a missed opportunity.
    ///
    /// ### Known problems
    /// - This lint is noisy! The `at_least_one_field` and `ignore` options (see below) can be used
    ///   to make the lint less noisy.
    /// - Currently does not support traits with type or constant parameters (e.g., `PartialEq`), or
    ///   traits with supertraits with type or constant parameters (e.g., `Eq`).
    ///
    /// ### Example
    /// ```rust
    /// #[derive(Default)]
    /// struct S;
    ///
    /// struct T(S);
    /// ```
    /// Use instead:
    /// ```rust
    /// #[derive(Default)]
    /// struct S;
    ///
    /// #[derive(Default)]
    /// struct T(S);
    /// ```
    ///
    /// ### Configuration
    /// - `at_least_one_field: bool` (default `false`): If set to `true`, the lint suggests to
    ///   derive a trait only when there is at least one field that implements (or could derive) the
    ///   trait.
    /// - `ignore: Vec<String>` (default `[]`): A list of macro paths the lint should not suggest to
    ///   derive.
    pub DERIVE_OPPORTUNITY,
    Warn,
    "data structures that could derive additional traits"
}

impl_lint_pass!(DeriveOpportunity<'_> => [DERIVE_OPPORTUNITY]);

#[allow(clippy::no_mangle_with_rust_abi)]
#[no_mangle]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[DERIVE_OPPORTUNITY]);
    lint_store.register_late_pass(move |_| Box::new(DeriveOpportunity::new()));
}

#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    at_least_one_field: bool,

    #[serde(default)]
    ignore: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Macro {
    Builtin(Symbol),
    External(DefId),
}

struct DeriveOpportunity<'tcx> {
    config: Config,
    derivable_traits_map: OnceCell<FxHashMap<Macro, FxHashSet<DefId>>>,
    transitively_applicable_macros_map: RefCell<FxHashMap<ty::Ty<'tcx>, FxHashSet<Macro>>>,
}

impl<'tcx> DeriveOpportunity<'tcx> {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
            derivable_traits_map: OnceCell::default(),
            transitively_applicable_macros_map: RefCell::new(FxHashMap::default()),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for DeriveOpportunity<'tcx> {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Enum(..) | ItemKind::Struct(..),) {
            return;
        }
        let ty = cx.tcx.type_of(item.owner_id).skip_binder();
        let macros = self
            .transitively_applicable_macros(cx, ty)
            .iter()
            .copied()
            .filter(|&mac| !self.macro_applied(cx, ty, mac))
            .collect::<Vec<_>>();
        if !macros.is_empty() {
            let mut paths = macros
                .into_iter()
                .map(|mac| mac.path(cx))
                .collect::<Vec<_>>();
            paths.sort();
            paths.dedup();
            let mut sugg = format!("#[derive({})]\n", paths.join(", "));
            if let Some(indent) = snippet_indent(cx, item.span) {
                sugg += &indent;
            }
            span_lint_and_sugg(
                cx,
                DERIVE_OPPORTUNITY,
                item.span.with_hi(item.span.lo()),
                "data structure could derive additional traits",
                "precede with",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

impl<'tcx> DeriveOpportunity<'tcx> {
    fn transitively_applicable_macros(
        &self,
        cx: &LateContext<'tcx>,
        ty: ty::Ty<'tcx>,
    ) -> FxHashSet<Macro> {
        if let Some(macros) = self.transitively_applicable_macros_map.borrow().get(&ty) {
            return macros.clone();
        }
        if let ty::Adt(adt_def, substs) = ty.kind()
            && let Some(span) = cx.tcx.hir().span_if_local(adt_def.did())
            && !span.from_expansion()
        {
            let mut macros_applicable_to_all_fields = self
                .derivable_traits(cx)
                .keys()
                .copied()
                .collect::<FxHashSet<_>>();
            let mut traits_derivable_for_at_least_one_field = FxHashSet::default();
            for field_def in adt_def.all_fields() {
                let field_ty = field_def.ty(cx.tcx, substs);

                let field_applied_macros = self
                    .derivable_traits(cx)
                    .keys()
                    .copied()
                    .filter(|&mac| self.macro_applied(cx, field_ty, mac))
                    .collect::<FxHashSet<_>>();

                let field_transitively_applicable_macros =
                    self.transitively_applicable_macros(cx, field_ty);

                let field_macros = field_applied_macros
                    .union(&field_transitively_applicable_macros)
                    .copied()
                    .collect();

                if !matches!(field_ty.kind(), ty::Param(_)) {
                    macros_applicable_to_all_fields = macros_applicable_to_all_fields
                        .intersection(&field_macros)
                        .copied()
                        .collect();
                }

                traits_derivable_for_at_least_one_field = traits_derivable_for_at_least_one_field
                    .union(&field_macros)
                    .copied()
                    .collect();
            }

            let trait_ids = if self.config.at_least_one_field {
                macros_applicable_to_all_fields
                    .intersection(&traits_derivable_for_at_least_one_field)
                    .copied()
                    .collect()
            } else {
                macros_applicable_to_all_fields
            }
            .into_iter()
            // smoelius: Applying `Default` to an `enum` requires special treatment.
            .filter(|&mac| !ty.is_enum() || mac != Macro::Builtin(sym::Default))
            .collect();

            self.transitively_applicable_macros_map
                .borrow_mut()
                .insert(ty, trait_ids);

            self.transitively_applicable_macros_map
                .borrow()
                .get(&ty)
                .unwrap()
                .clone()
        } else {
            FxHashSet::default()
        }
    }

    // smoelius: A macro may implement more than one trait. If a type implements any of them,
    // assume the macro was already applied to the type.
    fn macro_applied(&self, cx: &LateContext<'tcx>, ty: ty::Ty<'tcx>, mac: Macro) -> bool {
        self.derivable_traits(cx)
            .get(&mac)
            .unwrap()
            .iter()
            .any(|&trait_id| implements_trait_with_bounds(cx, ty, trait_id))
    }

    fn derivable_traits(&self, cx: &LateContext<'tcx>) -> &FxHashMap<Macro, FxHashSet<DefId>> {
        self.derivable_traits_map.get_or_init(|| {
            let mut derivable_traits_map = FxHashMap::<_, FxHashSet<_>>::default();
            for trait_id in cx.tcx.all_traits() {
                if let Some(mac) = is_derivable(cx, trait_id)
                    && !self.config.ignore.contains(&mac.path(cx))
                {
                    derivable_traits_map
                        .entry(mac)
                        .or_default()
                        .insert(trait_id);
                }
            }
            let macros = derivable_traits_map.keys().copied().collect::<Vec<_>>();
            for mac in macros {
                if !derivable_traits_map
                    .get(&mac)
                    .unwrap()
                    .iter()
                    .all(|&trait_id| all_params_are_lifetimes(cx.tcx, trait_id))
                {
                    derivable_traits_map.remove(&mac);
                }
            }
            derivable_traits_map
        })
    }
}

fn all_params_are_lifetimes(tcx: ty::TyCtxt<'_>, trait_id: DefId) -> bool {
    iter::once(trait_id)
        .chain(super_traits_of(tcx, trait_id))
        .all(|trait_id| {
            let generics = tcx.generics_of(trait_id);
            generics.count() == generics.params.len()
                && generics
                    .params
                    .iter()
                    .skip(1)
                    .all(|param| matches!(param.kind, ty::GenericParamDefKind::Lifetime))
        })
}

// smoelius: `super_traits_of` is a near carbon copy of the method of the same name here:
// https://github.com/rust-lang/rust/blob/fbdef58414af2b3469bf4f0f83bb136945414b96/compiler/rustc_middle/src/ty/context.rs#L1582-L1606

/// Computes the def-ids of the transitive supertraits of `trait_def_id`. This (intentionally) does
/// not compute the full elaborated super-predicates but just the set of def-ids. It is used
/// to identify which traits may define a given associated type to help avoid cycle errors.
/// Returns a `DefId` iterator.
fn super_traits_of(tcx: ty::TyCtxt<'_>, trait_def_id: DefId) -> impl Iterator<Item = DefId> + '_ {
    let mut set = FxHashSet::default();
    let mut stack = vec![trait_def_id];

    set.insert(trait_def_id);

    iter::from_fn(move || -> Option<DefId> {
        let trait_did = stack.pop()?;
        let generic_predicates = tcx.super_predicates_of(trait_did);

        for (predicate, _) in generic_predicates.predicates {
            if let ty::ClauseKind::Trait(data) = predicate.kind().skip_binder() {
                if set.insert(data.def_id()) {
                    stack.push(data.def_id());
                }
            }
        }

        Some(trait_did)
    })
}

/// Determines whether `trait_id` is derivable by checking whether any of its _known_ impls is
/// derived. (smoelius: Not ideal, but it's the best I've got for now.)
fn is_derivable(cx: &LateContext<'_>, trait_id: DefId) -> Option<Macro> {
    let impls = cx.tcx.trait_impls_of(trait_id);
    impls
        .blanket_impls()
        .iter()
        .chain(impls.non_blanket_impls().values().flatten())
        .find_map(|&def_id| is_derived(cx, def_id))
}

// smoelius: `is_derived` is based on `is_builtin_derived`:
// https://github.com/rust-lang/rust/blob/90f642bb3d74ee0ba8e0faf967748f36ff78d572/compiler/rustc_middle/src/ty/mod.rs#L2439-L2452
fn is_derived(cx: &LateContext<'_>, def_id: DefId) -> Option<Macro> {
    if let Some(def_id) = def_id.as_local()
        && let outer = cx.tcx.def_span(def_id).ctxt().outer_expn_data()
        && matches!(outer.kind, ExpnKind::Macro(MacroKind::Derive, _))
    {
        let macro_def_id = outer.macro_def_id.unwrap();
        if cx.tcx.has_attr(macro_def_id, sym::rustc_builtin_macro) {
            // smoelius: I'm not sure whether `SyntaxExtension::builtin_name` would be the right
            // thing to use here; regardless, I can't figure out how to retrieve that data:
            // https://github.com/rust-lang/rust/blob/d651fa78cefecefa87fa3d7dc1e1389d275afb63/compiler/rustc_expand/src/base.rs#L729-L731
            Some(Macro::Builtin(
                *cx.get_def_path(macro_def_id).last().unwrap(),
            ))
        } else {
            Some(Macro::External(macro_def_id))
        }
    } else {
        None
    }
}

fn implements_trait_with_bounds<'tcx>(
    cx: &LateContext<'tcx>,
    ty: ty::Ty<'tcx>,
    trait_id: DefId,
) -> bool {
    let generics = cx.tcx.generics_of(trait_id);
    // smoelius: `all_params_are_lifetimes` should have already been checked.
    let args = vec![
        ty::Region::new_from_kind(cx.tcx, ty::ReStatic).into();
        generics.params.len().saturating_sub(1)
    ];
    if let ty::Adt(adt_def, _) = ty.kind() {
        let param_env = param_env_with_bounds(cx.tcx, adt_def.did(), trait_id);
        // smoelius: The decision to pass `adt_def.did()` as the `callee_id` argument is based on
        // the following, but I am not sure it is the correct choice:
        // https://github.com/rust-lang/rust-clippy/blob/782520088f9c5a0274459060a6fdcd41301f35e2/clippy_lints/src/derive.rs#L453
        // See also: https://github.com/rust-lang/rust/pull/118661#discussion_r1449013176
        // smoelius: `Some(adt_def.did())` was changed to `None`. See:
        // https://github.com/rust-lang/rust/pull/120000
        implements_trait_with_env(cx.tcx, param_env, ty, trait_id, None, &args)
    } else {
        implements_trait(cx, ty, trait_id, &args)
    }
}

// smoelius: `param_env_with_bounds` is based on Clippy's `param_env_for_derived_eq`:
// https://github.com/rust-lang/rust-clippy/blob/716c552632acb50a524e62284b9ca2446333a626/clippy_lints/src/derive.rs#L493-L529

/// Creates the `ParamEnv` used for the give type's derived impl.
fn param_env_with_bounds(tcx: ty::TyCtxt<'_>, did: DefId, trait_id: DefId) -> ty::ParamEnv<'_> {
    // Initial map from generic index to param def.
    // Vec<(param_def, needs_bound)>
    let mut params = tcx
        .generics_of(did)
        .params
        .iter()
        .map(|p| (p, matches!(p.kind, ty::GenericParamDefKind::Type { .. })))
        .collect::<Vec<_>>();

    let ty_predicates = tcx.predicates_of(did).predicates;
    for (p, _) in ty_predicates {
        if let ty::ClauseKind::Trait(p) = p.kind().skip_binder()
            && p.trait_ref.def_id == trait_id
            && let ty::Param(self_ty) = p.trait_ref.self_ty().kind()
        {
            // Flag types which already have a bound.
            params[self_ty.index as usize].1 = false;
        }
    }

    ty::ParamEnv::new(
        tcx.mk_clauses_from_iter(
            ty_predicates.iter().map(|&(p, _)| p).chain(
                params
                    .iter()
                    .filter(|&&(_, needs_bound)| needs_bound)
                    .map(|&(param, _)| {
                        ty::ClauseKind::Trait(ty::TraitPredicate {
                            trait_ref: ty::TraitRef::new(
                                tcx,
                                trait_id,
                                [tcx.mk_param_from_def(param)],
                            ),
                            polarity: ty::PredicatePolarity::Positive,
                        })
                        .to_predicate(tcx)
                    }),
            ),
        ),
        Reveal::UserFacing,
    )
}

impl Macro {
    fn path(self, cx: &LateContext<'_>) -> String {
        match self {
            Self::Builtin(sym) => sym.to_string(),
            Self::External(def_id) => cx
                .get_def_path(def_id)
                .iter()
                .map(Symbol::as_str)
                .collect::<Vec<_>>()
                .join("::"),
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}

#[test]
fn ui_at_least_one_field() {
    dylint_testing::ui::Test::example(env!("CARGO_PKG_NAME"), "ui_at_least_one_field")
        .dylint_toml("derive_opportunity.at_least_one_field = true")
        .run();
}

#[test]
fn ui_ignore() {
    dylint_testing::ui::Test::example(env!("CARGO_PKG_NAME"), "ui_ignore")
        .dylint_toml(r#"derive_opportunity.ignore = ["serde_derive::Deserialize"]"#)
        .run();
}

#[test]
fn ui_main_rs_equal() {
    let ui_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/main.rs"),
    )
    .unwrap();
    let ui_at_least_one_field_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_at_least_one_field/main.rs"),
    )
    .unwrap();
    let ui_ignore_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_ignore/main.rs"),
    )
    .unwrap();
    assert_eq!(ui_main_rs, ui_at_least_one_field_main_rs);
    assert_eq!(ui_main_rs, ui_ignore_main_rs);
}
