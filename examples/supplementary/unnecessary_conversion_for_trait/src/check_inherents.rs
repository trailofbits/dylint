use super::{IGNORED_INHERENTS, WATCHED_INHERENTS};
use clippy_utils::paths::{PathNS, lookup_path};
use dylint_internal::match_def_path;
use rustc_hir::{Safety, def_id::DefId};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, TypeFolder, fast_reject::SimplifiedType};
use rustc_span::{Symbol, symbol::sym};

#[expect(clippy::too_many_lines)]
pub fn check_inherents(cx: &LateContext<'_>) {
    let into_iterator_def_id = cx.tcx.get_diagnostic_item(sym::IntoIterator).unwrap();
    let iterator_def_id = cx.tcx.get_diagnostic_item(sym::Iterator).unwrap();

    let type_paths = type_paths();

    let of_interest = |def_id| -> bool {
        if cx.tcx.visibility(def_id) != ty::Visibility::Public {
            return false;
        }

        let assoc_item = cx.tcx.associated_item(def_id);
        if !matches!(assoc_item.kind, ty::AssocKind::Fn { .. }) {
            return false;
        }

        let fn_sig = cx.tcx.fn_sig(assoc_item.def_id).skip_binder();
        if fn_sig.safety() == Safety::Unsafe || fn_sig.skip_binder().inputs().len() != 1 {
            return false;
        }

        let input_ty = fn_sig.input(0).skip_binder();
        let output_ty = fn_sig.output().skip_binder();

        if let Some(input_item_ty) = implements_trait_with_item(cx, input_ty, into_iterator_def_id)
        {
            if let Some(output_item_ty) = implements_trait_with_item(cx, output_ty, iterator_def_id)
                && input_item_ty == output_item_ty
            {
                return true;
            }
        } else {
            // smoelius: Sanity.
            assert!(!input_ty.to_string().starts_with("std::vec::Vec"));
        }

        [input_ty, output_ty].into_iter().all(|ty| {
            let ty = peel_unwanted(cx, def_id, ty);
            ty.is_slice()
                || ty.is_str()
                || ty.ty_adt_def().is_some_and(|adt_def| {
                    type_paths
                        .iter()
                        .any(|path| match_def_path(cx, adt_def.did(), path))
                })
        })
    };

    let type_path_impl_def_ids = type_paths
        .iter()
        .flat_map(|type_path| {
            lookup_path(
                cx.tcx,
                PathNS::Type,
                &type_path
                    .iter()
                    .copied()
                    .map(Symbol::intern)
                    .collect::<Vec<_>>(),
            )
        })
        .flat_map(|def_id| cx.tcx.inherent_impls(def_id));

    let slice_incoherent_impl_def_ids = cx
        .tcx
        .incoherent_impls(SimplifiedType::Slice)
        .iter()
        .filter(|&impl_def_id| {
            // smoelius: Filter out cases like `core::slice::ascii::<impl [u8]>::trim_ascii`.
            let ty::Slice(ty) = cx.tcx.type_of(impl_def_id).skip_binder().kind() else {
                panic!("impl is not for a slice");
            };
            matches!(ty.kind(), ty::Param(_))
        });

    let str_incoherent_impl_def_ids = cx.tcx.incoherent_impls(SimplifiedType::Str);

    let impl_def_ids = type_path_impl_def_ids
        .chain(slice_incoherent_impl_def_ids)
        .chain(str_incoherent_impl_def_ids)
        .copied()
        .collect::<Vec<_>>();

    // smoelius: Watched and ignored inherents are "of interest."
    for path in WATCHED_INHERENTS.iter().chain(IGNORED_INHERENTS.iter()) {
        if is_primitive_impl(path) || path.first() == Some(&"tempfile") {
            continue;
        }

        let def_id = lookup_path(
            cx.tcx,
            PathNS::Value,
            &path.iter().copied().map(Symbol::intern).collect::<Vec<_>>(),
        )
        .into_iter()
        .next()
        .ok_or_else(|| format!("`lookup_path_str` failed for {path:?}"))
        .unwrap();

        assert!(
            of_interest(def_id),
            "{:?} is not of interest",
            cx.get_def_path(def_id)
        );
    }

    // smoelius: Watched inherents are complete(ish).
    for &impl_def_id in &impl_def_ids {
        for &assoc_item_def_id in cx.tcx.associated_item_def_ids(impl_def_id) {
            if of_interest(assoc_item_def_id) {
                assert!(
                    WATCHED_INHERENTS
                        .iter()
                        .chain(IGNORED_INHERENTS.iter())
                        .any(|path| match_def_path(cx, assoc_item_def_id, path)),
                    "{:?} is missing",
                    cx.get_def_path(assoc_item_def_id)
                );
            }
        }
    }

    // smoelius: Every watched inherent satisfies one of the following three conditions:
    // - It is associated with one of the `type_paths` impls.
    // - It is associated with an incoherent impl.
    // - It is from the `tempfile` crate.
    let mut watched_inherents = WATCHED_INHERENTS.to_vec();
    for &impl_def_id in &impl_def_ids {
        for &assoc_item_def_id in cx.tcx.associated_item_def_ids(impl_def_id) {
            if let Some(i) = watched_inherents.iter().position(|&path| {
                path == cx
                    .get_def_path(assoc_item_def_id)
                    .iter()
                    .map(Symbol::as_str)
                    .collect::<Vec<_>>()
            }) {
                watched_inherents.remove(i);
            }
        }
    }
    assert!(
        watched_inherents
            .iter()
            .all(|path| path.first() == Some(&"tempfile")),
        "{watched_inherents:?}",
    );
}

fn type_paths() -> Vec<&'static [&'static str]> {
    let mut type_paths = WATCHED_INHERENTS
        .iter()
        .filter_map(|path| {
            // smoelius: `tempfile` must be filtered out because `def_path_res` does not handle it.
            if is_primitive_impl(path) || path.first() == Some(&"tempfile") {
                return None;
            }
            Some(path.split_last().unwrap().1)
        })
        .collect::<Vec<_>>();

    type_paths.dedup();

    type_paths
}

fn is_primitive_impl(path: &[&str]) -> bool {
    path.iter().any(|s| s.starts_with('<'))
}

// smoelius: See comment preceding `replace_ty_params_with_global_ty` re type parameters. If `ty`
// contains any constant parameters, `implements_trait_with_item` returns `None`.
fn implements_trait_with_item<'tcx>(
    cx: &LateContext<'tcx>,
    ty: ty::Ty<'tcx>,
    trait_id: DefId,
) -> Option<ty::Ty<'tcx>> {
    if let Some(adt_def) = ty.ty_adt_def()
        && cx
            .tcx
            .generics_of(adt_def.did())
            .own_params
            .iter()
            .any(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
    {
        return None;
    }

    cx.get_associated_type(
        replace_ty_params_with_global_ty(cx, ty),
        trait_id,
        sym::Item,
    )
}

// smoelius: This is a hack. For `get_associated_type` to return `Some(..)`, all of its argument
// type's type parameters must be substituted for. One of the types of interest is `Vec`, and its
// second type parameter must implement `alloc::alloc::Allocator`. So we instantiate all type
// parameters with the default `Allocator`, `alloc::alloc::Global`. A more robust solution would
// at least consider trait bounds and alert when a trait other than `Allocator` was encountered.
fn replace_ty_params_with_global_ty<'tcx>(
    cx: &LateContext<'tcx>,
    ty: ty::Ty<'tcx>,
) -> ty::Ty<'tcx> {
    let global_def_id = lookup_path(
        cx.tcx,
        PathNS::Type,
        &["alloc", "alloc", "Global"]
            .into_iter()
            .map(Symbol::intern)
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .next()
    .unwrap();
    let global_adt_def = cx.tcx.adt_def(global_def_id);
    let global_ty = ty::Ty::new_adt(cx.tcx, global_adt_def, ty::List::empty());
    ty::BottomUpFolder {
        tcx: cx.tcx,
        ty_op: |ty| {
            if matches!(ty.kind(), ty::Param(_)) {
                global_ty
            } else {
                ty
            }
        },
        lt_op: std::convert::identity,
        ct_op: std::convert::identity,
    }
    .fold_ty(ty)
}

fn peel_unwanted<'tcx>(
    cx: &LateContext<'tcx>,
    def_id: DefId,
    mut ty: ty::Ty<'tcx>,
) -> ty::Ty<'tcx> {
    const BOX: [&str; 3] = ["alloc", "boxed", "Box"];

    loop {
        match ty.kind() {
            ty::Ref(_, referent_ty, _) => {
                ty = *referent_ty;
                continue;
            }
            ty::Adt(adt_def, substs) if match_def_path(cx, adt_def.did(), &BOX) => {
                ty = substs[0].expect_ty();
                continue;
            }
            _ => {}
        }

        if let Some(as_ref_ty) = strip_as_ref(cx, def_id, ty) {
            ty = as_ref_ty;
            continue;
        }

        break;
    }

    ty
}

fn strip_as_ref<'tcx>(
    cx: &LateContext<'tcx>,
    def_id: DefId,
    ty: ty::Ty<'tcx>,
) -> Option<ty::Ty<'tcx>> {
    cx.tcx
        .param_env(def_id)
        .caller_bounds()
        .iter()
        .find_map(|predicate| {
            if let ty::ClauseKind::Trait(ty::TraitPredicate { trait_ref, .. }) =
                predicate.kind().skip_binder()
                && cx.tcx.get_diagnostic_item(sym::AsRef) == Some(trait_ref.def_id)
                && let [self_arg, subst_arg] = trait_ref.args.as_slice()
                && self_arg.kind() == ty::GenericArgKind::Type(ty)
                && let ty::GenericArgKind::Type(subst_ty) = subst_arg.kind()
            {
                Some(subst_ty)
            } else {
                None
            }
        })
}
