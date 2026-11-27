use std::any::type_name;

#[derive(Debug)]
pub struct Binding {
    type_name: &'static str,
    span: rustc_span::Span,
}

impl Binding {
    pub(crate) fn new<T>(span: rustc_span::Span) -> Self {
        Self {
            type_name: type_name::<T>(),
            span,
        }
    }

    pub(crate) fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub(crate) fn span(&self) -> rustc_span::Span {
        self.span
    }
}
