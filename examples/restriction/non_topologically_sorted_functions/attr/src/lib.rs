use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse_str;

#[proc_macro_attribute]
pub fn attr(_args: TokenStream, tokens: TokenStream) -> TokenStream {
    let tokens = parse_str::<TokenStream2>(&tokens.to_string()).unwrap();
    tokens.into()
}
