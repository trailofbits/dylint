use proc_macro2::LexError;
use syn::parse::Error as ParseError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub struct Error {
    span: rustc_span::Span,
    kind: ErrorKind,
}

impl Error {
    pub fn new(span: rustc_span::Span, error: impl Into<ErrorKind>) -> Self {
        Self {
            span,
            kind: error.into(),
        }
    }

    pub fn other(span: rustc_span::Span, error: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self {
            span,
            kind: error.into().into(),
        }
    }

    #[must_use]
    pub fn span(&self) -> rustc_span::Span {
        self.span
    }

    #[must_use]
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}: {}", self.span, self.kind))
    }
}

// smoelius: Please keep this list sorted by order in which the errors could occur.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum ErrorKind {
    #[error("lex error: {0}")]
    LexError(#[from] LexError),

    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),

    #[error("failed to get span")]
    NoSpan,

    #[error("failed to get source text")]
    NoSource,

    #[error("failed to match pattern")]
    NoMatch,

    #[error("found no `HirId` for `{type_name}`")]
    NoHirId { type_name: &'static str },

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}
