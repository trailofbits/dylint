use paste::paste;

pub trait Visitable {
    fn visit<'ast, V: syn::visit::Visit<'ast>>(&'ast self, _visitor: &mut V) {}
    fn visit_children<'ast, V: syn::visit::Visit<'ast>>(&'ast self, _visitor: &mut V) {}
}

macro_rules! impl_visitable {
    ($ty:ident, $ty_snake:ident) => {
        paste! {
            impl Visitable for syn::$ty {
                fn visit<'ast, V: syn::visit::Visit<'ast>>(&'ast self, visitor: &mut V) {
                    visitor.[< visit_ $ty_snake >](self);
                }
                fn visit_children<'ast, V: syn::visit::Visit<'ast>>(&'ast self, visitor: &mut V) {
                    syn::visit::[< visit_ $ty_snake >](visitor, self);
                }
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/visitable.rs"));
