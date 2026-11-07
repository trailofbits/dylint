use crate::HirToSyn;
use rustc_hir as hir;
use std::marker::PhantomData;

// smoelius: `TypeNameGetter` uses Nikolai Vazquez's trick from `impls`. See:
// https://github.com/nvzqz/impls#how-it-works

struct TypeNameGetter<T>(PhantomData<T>);

trait HirToSynUnimplemented {
    fn type_name() -> Option<&'static str>;
}

impl<T> HirToSynUnimplemented for TypeNameGetter<T> {
    /// If `HirToSyn` is not implemented for `T`, `type_name` will resolve to this trait method.
    fn type_name() -> Option<&'static str> {
        None
    }
}

impl<T: HirToSyn> TypeNameGetter<T> {
    /// If `HirToSyn` is implemented for `T`, `type_name` will resolve to this inherent method.
    #[allow(clippy::unnecessary_wraps)]
    fn type_name() -> Option<&'static str> {
        Some(std::any::type_name::<T::Syn>())
    }
}

#[test]
fn sanity_implemented() {
    assert_eq!(
        Some("syn::expr::Expr"),
        TypeNameGetter::<hir::AnonConst>::type_name()
    );
}

#[test]
fn sanity_unimplemented() {
    assert_eq!(None, TypeNameGetter::<hir::FnPtrTy>::type_name());
}

include!(concat!(env!("OUT_DIR"), "/type_name.rs"));
