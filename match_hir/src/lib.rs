#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[allow(unused_extern_crates)]
extern crate rustc_driver;

use proc_macro2::TokenStream as TokenStream2;
use rustc_hir::HirId;
use rustc_lint::LateContext;
use std::str::FromStr;

pub use syn::parse::Parse;

mod binding;
use binding::Binding;

mod clippy_utils;
use clippy_utils::get_source_text;
pub use clippy_utils::snippet_opt as __snippet_opt;

mod error;
pub use error::{Error, ErrorKind};

mod hir_node;
pub use hir_node::HirNode;

mod span_hir_id_map;
pub use span_hir_id_map::__hir_ids_from_span_untyped;
use span_hir_id_map::{
    __hir_ids_from_span_untyped as hir_ids_from_span_untyped, hir_ids_from_span,
};

mod syntax;
pub use syntax::HirToSyn;

mod toxic;
pub use toxic::Unify;
use toxic::variables;

mod type_name;
use type_name::type_name;

mod visitable;
use visitable::Visitable;

#[derive(Clone)]
pub struct Pattern {
    stream: TokenStream2,
    n_vars: usize,
}

impl Pattern {
    #[allow(clippy::unnecessary_wraps)]
    fn new(stream: TokenStream2) -> Result<Self, Error> {
        let (stream, n_vars) = variables::mark(stream);
        Ok(Self { stream, n_vars })
    }
}

impl FromStr for Pattern {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream =
            TokenStream2::from_str(s).map_err(|error| Error::new(rustc_span::DUMMY_SP, error))?;
        Self::new(stream)
    }
}

impl Pattern {
    pub fn matches<T>(&self, cx: &LateContext, hir: &T) -> Result<Vec<HirId>, Error>
    where
        T: HirNode + HirToSyn,
    {
        let hir_id = hir.hir_id();

        self.matches_hir_id::<T::Syn>(cx, hir_id)
    }

    pub fn matches_hir_id<T>(&self, cx: &LateContext, hir_id: HirId) -> Result<Vec<HirId>, Error>
    where
        T: Parse + Unify,
    {
        let bindings = self.matches_hir_id_inner::<T>(cx, hir_id)?;

        // smoelius: Re the use of `first`, see the comment preceding `matches_hir_id_inner`'s
        // definition.
        Ok(bindings
            .into_iter()
            .map(|hir_ids| hir_ids.first().copied().unwrap())
            .collect())
    }

    // smoelius: Note that `matches_hir_id_inner` returns a vector of vectors to support a
    // "multi-matches" interface. However, I am on the fence as to whether we should provide that
    // interface.
    fn matches_hir_id_inner<T>(
        &self,
        cx: &LateContext,
        hir_id: HirId,
    ) -> Result<Vec<Vec<HirId>>, Error>
    where
        T: Parse + Unify,
    {
        #[allow(clippy::disallowed_methods)]
        let span = cx.tcx.hir_span(hir_id);

        if !hir_ids_from_span_untyped(cx, span).contains(&hir_id) {
            return Err(Error::new(span, ErrorKind::NoSpan));
        }

        let mut bindings = Vec::with_capacity(self.n_vars);

        let scrutinee: T = reparse_as_syn(cx, span)?;
        let pattern: T =
            syn::parse2(self.stream.clone()).map_err(|error| Error::new(span, error))?;

        scrutinee.unify(span, &pattern, &mut bindings)?;

        assert_eq!(self.n_vars, bindings.len());

        bindings
            .into_iter()
            .map(|binding| {
                let hir_ids = hir_ids_from_span::<T>(cx, binding.span());
                if hir_ids.is_empty() {
                    return Err(Error::new(
                        span,
                        ErrorKind::NoHirId {
                            type_name: binding.type_name(),
                        },
                    ));
                }
                Ok(hir_ids)
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

fn reparse_as_syn<T: Parse>(cx: &LateContext, span: rustc_span::Span) -> Result<T, Error> {
    if let Some(source_file_range) = get_source_text(cx, span)
        && let Some(text) = source_file_range.as_str()
    {
        syn::parse_str::<T>(text).map_err(|error| Error::new(span, ErrorKind::ParseError(error)))
    } else {
        Err(Error::new(span, ErrorKind::NoSource))
    }
}
