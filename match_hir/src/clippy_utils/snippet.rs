use rustc_lint::LintContext;
use rustc_session::Session;
use rustc_span::Span;

// smoelius: Everything below this comment was copied from:
// https://github.com/rust-lang/rust/blob/e0eba9cafcc8aaf3821f4b0b9777954caf049498/clippy_utils/src/source.rs#L238-L245

/// Converts a span to a code snippet. Returns `None` if not available.
pub fn snippet_opt(cx: &impl LintContext, span: Span) -> Option<String> {
    snippet_opt_sess(cx.sess(), span)
}

fn snippet_opt_sess(sess: &Session, span: Span) -> Option<String> {
    sess.source_map().span_to_snippet(span).ok()
}
