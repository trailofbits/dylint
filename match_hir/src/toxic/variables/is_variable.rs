use super::MARKER;

pub(crate) trait IsVariable {
    fn is_variable(&self) -> bool {
        false
    }
}

impl IsVariable for syn::Expr {
    fn is_variable(&self) -> bool {
        if let syn::Expr::Path(syn::ExprPath { attrs, qself, path }) = self
            && attrs.is_empty()
            && qself.is_none()
        {
            path.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::Ident {
    fn is_variable(&self) -> bool {
        *self == MARKER
    }
}

impl IsVariable for syn::Pat {
    fn is_variable(&self) -> bool {
        if let syn::Pat::Ident(syn::PatIdent {
            attrs,
            by_ref,
            mutability,
            ident,
            subpat,
        }) = self
            && attrs.is_empty()
            && by_ref.is_none()
            && mutability.is_none()
            && subpat.is_none()
        {
            ident.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::Path {
    fn is_variable(&self) -> bool {
        if let Some(ident) = self.get_ident() {
            ident.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::PathSegment {
    fn is_variable(&self) -> bool {
        self.ident.is_variable() && self.arguments.is_empty()
    }
}

impl IsVariable for syn::Stmt {
    fn is_variable(&self) -> bool {
        if let syn::Stmt::Expr(expr, _) = self {
            expr.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::Type {
    fn is_variable(&self) -> bool {
        if let syn::Type::Path(syn::TypePath { qself, path }) = self
            && qself.is_none()
        {
            path.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::GenericArgument {
    fn is_variable(&self) -> bool {
        if let syn::GenericArgument::Type(ty) = self {
            ty.is_variable()
        } else {
            false
        }
    }
}

impl IsVariable for syn::Member {
    fn is_variable(&self) -> bool {
        if let syn::Member::Named(ident) = self {
            ident.is_variable()
        } else {
            false
        }
    }
}
