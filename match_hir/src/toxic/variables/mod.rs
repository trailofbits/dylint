use proc_macro2::{Delimiter, Group, Spacing, Span, TokenStream as TokenStream2, TokenTree};

mod is_variable;
pub(crate) use is_variable::IsVariable;

const MARKER: &str = "__match_hir_variable__";

#[must_use]
pub(crate) fn mark(input: TokenStream2) -> (TokenStream2, usize) {
    let mut iter = input.into_iter().peekable();
    let mut output = Vec::new();
    let mut n: usize = 0;
    while let Some(tt) = iter.next() {
        match &tt {
            TokenTree::Punct(punct) => {
                if punct.as_char() == '#'
                    && punct.spacing() == Spacing::Alone
                    && let Some(TokenTree::Group(group)) = iter.peek()
                    && group.delimiter() == Delimiter::Parenthesis
                    // smoelius: Currently, we only check for/permit the wildcard character (`_`)
                    // inside the parens. In the future, we might want to allow the variable name
                    // inside the parens.
                    && let [token] = group.stream().into_iter().collect::<Vec<_>>().as_slice()
                    && let TokenTree::Ident(ident) = token
                    && *ident == "_"
                {
                    #[allow(let_underscore_drop, clippy::unwrap_used)]
                    let _ = iter.next().unwrap();
                    output.push(TokenTree::Ident(syn::Ident::new(MARKER, Span::call_site())));
                    n += 1;
                } else {
                    output.push(tt);
                }
            }
            TokenTree::Group(group) => {
                let (stream, m) = mark(group.stream());
                output.push(TokenTree::Group(Group::new(group.delimiter(), stream)));
                n += m;
            }
            _ => {
                output.push(tt);
            }
        }
    }
    (output.into_iter().collect(), n)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mark_variables() {
        let (stream, n) = super::mark(quote::quote! {
            let #(_) = #(_) + 1;
        });
        assert_eq!(
            quote::ToTokens::to_token_stream(&stream).to_string(),
            format!("let {MARKER} = {MARKER} + 1 ;")
        );
        assert_eq!(2, n);
    }
}
