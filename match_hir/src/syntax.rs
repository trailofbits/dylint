use crate::Unify;
use rustc_ast as ast;
use rustc_hir as hir;
use rustc_span as span;
use syn::{parse::Parse, spanned::Spanned};

pub trait HirToSyn {
    type Syn: Parse + Spanned + Unify;
}

macro_rules! impl_hir_to_syn {
    ($hir_ty:path, $syn_ty:ty) => {
        impl HirToSyn for $hir_ty {
            type Syn = $syn_ty;
        }
    };
}

// smoelius: The below list was generated manually, not automatically, and may be incomplete.
impl_hir_to_syn!(ast::Label, syn::Label);
impl_hir_to_syn!(hir::AnonConst, syn::Expr);
impl_hir_to_syn!(hir::Arm<'_>, syn::Arm);
impl_hir_to_syn!(hir::Block<'_>, syn::Block);
impl_hir_to_syn!(hir::ExprField<'_>, syn::FieldValue);
impl_hir_to_syn!(hir::Expr<'_>, syn::Expr);
impl_hir_to_syn!(hir::FnRetTy<'_>, syn::ReturnType);
impl_hir_to_syn!(hir::GenericArg<'_>, syn::GenericArgument);
impl_hir_to_syn!(hir::Lit, syn::Lit);
impl_hir_to_syn!(hir::ParamName, syn::Ident);
impl_hir_to_syn!(hir::Path<'_>, syn::Path);
impl_hir_to_syn!(hir::PathSegment<'_>, syn::PathSegment);
impl_hir_to_syn!(hir::Stmt<'_>, syn::Stmt);
impl_hir_to_syn!(hir::Ty<'_>, syn::Type);
impl_hir_to_syn!(hir::RangeEnd, syn::RangeLimits);
impl_hir_to_syn!(hir::UnOp, syn::UnOp);
impl_hir_to_syn!(span::symbol::Ident, syn::Ident);
