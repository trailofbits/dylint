use rustc_hir::{self as hir, HirId};

/// A type that knows its `HirId`.
///
/// Similar to [`clippy_utils::HirNode`].
///
/// [`clippy_utils::HirNode`]: https://github.com/rust-lang/rust/blob/786f874c349b995ebed5e3d8f20db1cf65f20782/clippy_utils/src/macros.rs#L510-L539
pub trait HirNode {
    fn hir_id(&self) -> HirId;
}

macro_rules! impl_hir_node_with_method {
    ($path:path) => {
        impl HirNode for $path {
            fn hir_id(&self) -> HirId {
                self.hir_id()
            }
        }
    };
}

macro_rules! impl_hir_node_with_struct_field {
    ($path:path) => {
        impl HirNode for $path {
            fn hir_id(&self) -> HirId {
                self.hir_id
            }
        }
    };
}

// smoelius: The below lists were generated manually, not automatically, and may be incomplete.

impl_hir_node_with_method!(hir::ForeignItem<'_>);
impl_hir_node_with_method!(hir::ForeignItemId);
impl_hir_node_with_method!(hir::GenericArg<'_>);
impl_hir_node_with_method!(hir::ImplItem<'_>);
impl_hir_node_with_method!(hir::ImplItemId);
impl_hir_node_with_method!(hir::Item<'_>);
impl_hir_node_with_method!(hir::ItemId);
impl_hir_node_with_method!(hir::TraitItem<'_>);
impl_hir_node_with_method!(hir::TraitItemId);

// smoelius: ??? warning: function cannot return without recursing
// impl_hir_node_with_method!(hir::PreciseCapturingArg<'_>);

impl_hir_node_with_struct_field!(hir::AnonConst);
impl_hir_node_with_struct_field!(hir::Arm<'_>);
impl_hir_node_with_struct_field!(hir::AssocItemConstraint<'_>);
impl_hir_node_with_struct_field!(hir::Block<'_>);
impl_hir_node_with_struct_field!(hir::BodyId);
impl_hir_node_with_struct_field!(hir::ConstArg<'_>);
impl_hir_node_with_struct_field!(hir::ConstBlock);
impl_hir_node_with_struct_field!(hir::Expr<'_>);
impl_hir_node_with_struct_field!(hir::ExprField<'_>);
impl_hir_node_with_struct_field!(hir::FieldDef<'_>);
impl_hir_node_with_struct_field!(hir::GenericParam<'_>);
impl_hir_node_with_struct_field!(hir::InferArg);
impl_hir_node_with_struct_field!(hir::LetStmt<'_>);
impl_hir_node_with_struct_field!(hir::Lifetime);
impl_hir_node_with_struct_field!(hir::OpaqueTy<'_>);
impl_hir_node_with_struct_field!(hir::Param<'_>);
impl_hir_node_with_struct_field!(hir::Pat<'_>);
impl_hir_node_with_struct_field!(hir::PatExpr<'_>);
impl_hir_node_with_struct_field!(hir::PatField<'_>);
impl_hir_node_with_struct_field!(hir::PathSegment<'_>);
impl_hir_node_with_struct_field!(hir::PreciseCapturingNonLifetimeArg);
impl_hir_node_with_struct_field!(hir::Stmt<'_>);
impl_hir_node_with_struct_field!(hir::Ty<'_>);
impl_hir_node_with_struct_field!(hir::TyPat<'_>);
impl_hir_node_with_struct_field!(hir::Variant<'_>);
impl_hir_node_with_struct_field!(hir::WherePredicate<'_>);
