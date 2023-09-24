use marker_api::ast::{
    ty::{
        CommonSynTyData, NumKind, SemAdtTy, SemAliasTy, SemArrayTy, SemBoolTy, SemClosureTy,
        SemFnPtrTy, SemFnTy, SemGenericTy, SemNeverTy, SemNumTy, SemRawPtrTy, SemRefTy, SemSliceTy,
        SemTextTy, SemTraitObjTy, SemTupleTy, SemTyKind, SemUnstableTy, SynArrayTy, SynBoolTy,
        SynFnPtrTy, SynImplTraitTy, SynInferredTy, SynNeverTy, SynNumTy, SynPathTy, SynRawPtrTy,
        SynRefTy, SynSliceTy, SynTextTy, SynTraitObjTy, SynTupleTy, SynTyKind, TextKind,
    },
    CommonCallableData, ConstValue, Constness, Parameter, Syncness,
};
use rustc_hir as hir;
use rustc_middle as mid;

use super::MarkerConverterInner;

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_sem_ty(&self, rustc_ty: mid::ty::Ty<'tcx>) -> SemTyKind<'ast> {
        // Semantic types could be cached, the question is if they should and at
        // which level.
        match &rustc_ty.kind() {
            mid::ty::TyKind::Bool => SemTyKind::Bool(self.alloc(SemBoolTy::new())),
            mid::ty::TyKind::Char => SemTyKind::Text(self.alloc(SemTextTy::new(TextKind::Char))),
            mid::ty::TyKind::Int(int_ty) => {
                let num_ty = match int_ty {
                    mid::ty::IntTy::Isize => NumKind::Isize,
                    mid::ty::IntTy::I8 => NumKind::I8,
                    mid::ty::IntTy::I16 => NumKind::I16,
                    mid::ty::IntTy::I32 => NumKind::I32,
                    mid::ty::IntTy::I64 => NumKind::I64,
                    mid::ty::IntTy::I128 => NumKind::I128,
                };
                SemTyKind::Num(self.alloc(SemNumTy::new(num_ty)))
            }
            mid::ty::TyKind::Uint(uint_ty) => {
                let num_ty = match uint_ty {
                    mid::ty::UintTy::Usize => NumKind::Usize,
                    mid::ty::UintTy::U8 => NumKind::U8,
                    mid::ty::UintTy::U16 => NumKind::U16,
                    mid::ty::UintTy::U32 => NumKind::U32,
                    mid::ty::UintTy::U64 => NumKind::U64,
                    mid::ty::UintTy::U128 => NumKind::U128,
                };
                SemTyKind::Num(self.alloc(SemNumTy::new(num_ty)))
            }
            mid::ty::TyKind::Float(float_ty) => {
                let num_ty = match float_ty {
                    mid::ty::FloatTy::F32 => NumKind::F32,
                    mid::ty::FloatTy::F64 => NumKind::F64,
                };
                SemTyKind::Num(self.alloc(SemNumTy::new(num_ty)))
            }
            mid::ty::TyKind::Str => SemTyKind::Text(self.alloc(SemTextTy::new(TextKind::Str))),
            mid::ty::TyKind::Adt(def, generics) => SemTyKind::Adt(self.alloc(SemAdtTy::new(
                self.to_ty_def_id(def.did()),
                self.to_sem_generic_args(generics),
            ))),
            mid::ty::TyKind::Foreign(_) => {
                todo!("foreign type are currently sadly not supported. See rust-marker/marker#182")
            }
            mid::ty::TyKind::Array(inner, _len) => SemTyKind::Array(
                self.alloc(SemArrayTy::new(self.to_sem_ty(*inner), ConstValue::new())),
            ),
            mid::ty::TyKind::Slice(inner) => {
                SemTyKind::Slice(self.alloc(SemSliceTy::new(self.to_sem_ty(*inner))))
            }
            mid::ty::TyKind::Tuple(ty_lst) => SemTyKind::Tuple(self.alloc(SemTupleTy::new(
                self.alloc_slice(ty_lst.iter().map(|ty| self.to_sem_ty(ty))),
            ))),
            mid::ty::TyKind::RawPtr(ty_and_mut) => SemTyKind::RawPtr(self.alloc(SemRawPtrTy::new(
                self.to_mutability(ty_and_mut.mutbl),
                self.to_sem_ty(ty_and_mut.ty),
            ))),
            mid::ty::TyKind::Ref(_lifetime, inner, muta) => SemTyKind::Ref(self.alloc(
                SemRefTy::new(self.to_mutability(*muta), self.to_sem_ty(*inner)),
            )),
            mid::ty::TyKind::FnDef(fn_id, generic_args) => {
                SemTyKind::FnTy(self.alloc(SemFnTy::new(
                    self.to_item_id(*fn_id),
                    self.to_sem_generic_args(generic_args),
                )))
            }
            mid::ty::TyKind::FnPtr(fn_info) => SemTyKind::FnPtr(
                self.alloc(SemFnPtrTy::new(
                    self.to_safety(fn_info.unsafety()),
                    self.to_abi(fn_info.abi()),
                    self.alloc_slice(
                        fn_info
                            .inputs()
                            .skip_binder()
                            .iter()
                            .map(|input| self.to_sem_ty(*input)),
                    ),
                    self.to_sem_ty(fn_info.output().skip_binder()),
                )),
            ),
            mid::ty::TyKind::Dynamic(binders, _region, kind) => {
                if !matches!(kind, mid::ty::DynKind::Dyn) {
                    unimplemented!("the docs are not totally clear, when `DynStar` is used, her it is: {rustc_ty:#?}")
                }
                SemTyKind::TraitObj(
                    self.alloc(SemTraitObjTy::new(self.to_sem_trait_bounds(binders))),
                )
            }
            mid::ty::TyKind::Closure(id, generics) => SemTyKind::ClosureTy(self.alloc(
                SemClosureTy::new(self.to_ty_def_id(*id), self.to_sem_generic_args(generics)),
            )),
            mid::ty::TyKind::Generator(_, _, _)
            | mid::ty::TyKind::GeneratorWitness(_)
            | mid::ty::TyKind::GeneratorWitnessMIR(_, _) => {
                SemTyKind::Unstable(self.alloc(SemUnstableTy::new()))
            }
            mid::ty::TyKind::Never => SemTyKind::Never(self.alloc(SemNeverTy::new())),
            mid::ty::TyKind::Alias(_, info) => {
                SemTyKind::Alias(self.alloc(SemAliasTy::new(self.to_item_id(info.def_id))))
            }
            mid::ty::TyKind::Param(param) => {
                let body_id = self
                    .rustc_body
                    .borrow()
                    .expect("semantic `TyKind::Param` is only valid inside bodies");
                // This is a local id, this makes sense, since rustc only accesses
                // expressions and therefore semantic types of the current crate.
                // This should be fine...
                let owner = self.rustc_cx.hir().body_owner_def_id(body_id);
                let generic_info = self
                    .rustc_cx
                    .generics_of(owner.to_def_id())
                    .type_param(param, self.rustc_cx);
                SemTyKind::Generic(
                    self.alloc(SemGenericTy::new(self.to_generic_id(generic_info.def_id))),
                )
            }
            mid::ty::TyKind::Bound(_, _) => {
                unreachable!(
                    "used by rustc for higher ranked types, which are not represented in marker"
                )
            }
            mid::ty::TyKind::Placeholder(_) | mid::ty::TyKind::Infer(_) => {
                unreachable!("used by rustc during typechecking, should not exist afterwards")
            }
            mid::ty::TyKind::Error(_) => unreachable!("would have triggered a rustc error"),
        }
    }
}

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_syn_ty(&self, rustc_ty: &'tcx hir::Ty<'tcx>) -> SynTyKind<'ast> {
        let data = CommonSynTyData::new_syntactic(self.to_span_id(rustc_ty.span));

        // Note: Here we can't reuse allocated nodes, as each one contains
        // a unique span id. These nodes don't need to be stored individually, as
        // they can't be requested individually over the API. Instead, they're
        // always stored as part of a parent node.
        match &rustc_ty.kind {
            hir::TyKind::Slice(inner_ty) => {
                SynTyKind::Slice(self.alloc(SynSliceTy::new(data, self.to_syn_ty(inner_ty))))
            }
            hir::TyKind::Array(inner_ty, rust_len) => {
                let len = match rust_len {
                    hir::ArrayLen::Body(anon) => Some(self.to_const_expr(*anon)),
                    hir::ArrayLen::Infer(_, _) => None,
                };
                SynTyKind::Array(self.alloc(SynArrayTy::new(data, self.to_syn_ty(inner_ty), len)))
            }
            hir::TyKind::Ptr(mut_ty) => SynTyKind::RawPtr(self.alloc(SynRawPtrTy::new(
                data,
                self.to_mutability(mut_ty.mutbl),
                self.to_syn_ty(mut_ty.ty),
            ))),
            hir::TyKind::Ref(rust_lt, mut_ty) => SynTyKind::Ref(self.alloc({
                SynRefTy::new(
                    data,
                    self.to_lifetime(rust_lt),
                    self.to_mutability(mut_ty.mutbl),
                    self.to_syn_ty(mut_ty.ty),
                )
            })),
            hir::TyKind::BareFn(rust_fn) => {
                SynTyKind::FnPtr(self.alloc(self.to_syn_fn_prt_ty(data, rust_fn)))
            }
            hir::TyKind::Never => SynTyKind::Never(self.alloc(SynNeverTy::new(data))),
            hir::TyKind::Tup(rustc_tys) => {
                let api_tys =
                    self.alloc_slice(rustc_tys.iter().map(|rustc_ty| self.to_syn_ty(rustc_ty)));
                SynTyKind::Tuple(self.alloc(SynTupleTy::new(data, api_tys)))
            }
            hir::TyKind::Path(qpath) => self.to_syn_ty_from_qpath(data, qpath, rustc_ty),
            // Continue ty conversion
            hir::TyKind::Err(..) => unreachable!("would have triggered a rustc error"),
            hir::TyKind::Typeof(_) => unreachable!("docs state: 'Unused for now.'"),
            hir::TyKind::OpaqueDef(id, _, _) => {
                // `impl Trait` in rustc are implemented as Items with the kind `OpaqueTy`
                let item = self.rustc_cx.hir().item(*id);
                let hir::ItemKind::OpaqueTy(opty) = &item.kind else {
                    unreachable!("the item of a `OpaqueDef` should be `OpaqueTy` {item:#?}");
                };
                let rust_bound = self.to_syn_ty_param_bound(opty.bounds);
                // FIXME: Generics are a bit weird with opaque types
                SynTyKind::ImplTrait(self.alloc(SynImplTraitTy::new(data, rust_bound)))
            }
            hir::TyKind::TraitObject(rust_bounds, rust_lt, _syntax) => {
                SynTyKind::TraitObj(self.alloc(SynTraitObjTy::new(
                    data,
                    self.to_syn_ty_param_bound_from_hir(rust_bounds, rust_lt),
                )))
            }
            hir::TyKind::Infer => SynTyKind::Inferred(self.alloc(SynInferredTy::new(data))),
        }
    }

    #[must_use]
    pub fn to_syn_fn_prt_ty(
        &self,
        data: CommonSynTyData<'ast>,
        rust_fn: &hir::BareFnTy<'tcx>,
    ) -> SynFnPtrTy<'ast> {
        assert_eq!(rust_fn.param_names.len(), rust_fn.decl.inputs.len());
        let params = rust_fn
            .decl
            .inputs
            .iter()
            .zip(rust_fn.param_names.iter())
            .map(|(rustc_ty, name)| {
                Parameter::new(
                    Some(self.to_symbol_id(name.name)),
                    Some(self.to_syn_ty(rustc_ty)),
                    Some(self.to_span_id(name.span)),
                )
            });
        let params = self.alloc_slice(params);
        let return_ty = if let hir::FnRetTy::Return(rust_ty) = rust_fn.decl.output {
            Some(self.to_syn_ty(rust_ty))
        } else {
            None
        };
        SynFnPtrTy::new(
            data,
            CommonCallableData::new(
                Constness::NotConst,
                Syncness::Sync,
                self.to_safety(rust_fn.unsafety),
                false,
                self.to_abi(rust_fn.abi),
                false,
                params,
                return_ty,
            ),
        )
    }

    fn to_syn_ty_from_qpath(
        &self,
        data: CommonSynTyData<'ast>,
        qpath: &hir::QPath<'tcx>,
        rustc_ty: &hir::Ty<'_>,
    ) -> SynTyKind<'ast> {
        match qpath {
            hir::QPath::Resolved(_, path) => match path.res {
                hir::def::Res::Def(
                    hir::def::DefKind::LifetimeParam
                    | hir::def::DefKind::TyParam
                    | hir::def::DefKind::ConstParam
                    | hir::def::DefKind::TyAlias
                    | hir::def::DefKind::Enum
                    | hir::def::DefKind::Struct
                    | hir::def::DefKind::Union
                    | hir::def::DefKind::Trait
                    | hir::def::DefKind::AssocTy
                    | hir::def::DefKind::ForeignTy
                    | hir::def::DefKind::TraitAlias,
                    _,
                )
                | hir::def::Res::SelfTyParam { .. }
                | hir::def::Res::SelfTyAlias { .. } => SynTyKind::Path(
                    self.alloc(SynPathTy::new(data, self.to_qpath_from_ty(qpath, rustc_ty))),
                ),
                hir::def::Res::PrimTy(prim_ty) => self.to_syn_ty_from_prim_ty(data, prim_ty),
                hir::def::Res::Def(_, _)
                | hir::def::Res::SelfCtor(_)
                | hir::def::Res::Local(_)
                | hir::def::Res::ToolMod
                | hir::def::Res::NonMacroAttr(_) => unreachable!("not a syntactic type {path:#?}"),
                hir::def::Res::Err => unreachable!("would have triggered a rustc error"),
            },
            hir::QPath::TypeRelative(_, _) | hir::QPath::LangItem(_, _, _) => SynTyKind::Path(
                self.alloc(SynPathTy::new(data, self.to_qpath_from_ty(qpath, rustc_ty))),
            ),
        }
    }

    fn to_syn_ty_from_prim_ty(
        &self,
        data: CommonSynTyData<'ast>,
        prim_ty: hir::PrimTy,
    ) -> SynTyKind<'ast> {
        let num_kind = match prim_ty {
            hir::PrimTy::Int(int_ty) => match int_ty {
                rustc_ast::IntTy::Isize => NumKind::Isize,
                rustc_ast::IntTy::I8 => NumKind::I8,
                rustc_ast::IntTy::I16 => NumKind::I16,
                rustc_ast::IntTy::I32 => NumKind::I32,
                rustc_ast::IntTy::I64 => NumKind::I64,
                rustc_ast::IntTy::I128 => NumKind::I128,
            },
            hir::PrimTy::Uint(uint_ty) => match uint_ty {
                rustc_ast::UintTy::Usize => NumKind::Usize,
                rustc_ast::UintTy::U8 => NumKind::U8,
                rustc_ast::UintTy::U16 => NumKind::U16,
                rustc_ast::UintTy::U32 => NumKind::U32,
                rustc_ast::UintTy::U64 => NumKind::U64,
                rustc_ast::UintTy::U128 => NumKind::U128,
            },
            hir::PrimTy::Float(kind) => match kind {
                rustc_ast::FloatTy::F32 => NumKind::F32,
                rustc_ast::FloatTy::F64 => NumKind::F64,
            },
            hir::PrimTy::Str => {
                return SynTyKind::Text(self.alloc(SynTextTy::new(data, TextKind::Str)))
            }
            hir::PrimTy::Bool => return SynTyKind::Bool(self.alloc(SynBoolTy::new(data))),
            hir::PrimTy::Char => {
                return SynTyKind::Text(self.alloc(SynTextTy::new(data, TextKind::Char)));
            }
        };
        SynTyKind::Num(self.alloc(SynNumTy::new(data, num_kind)))
    }
}
