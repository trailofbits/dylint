use rustc_lint::LintContext;
use rustc_span::{BytePos, Pos, SourceFile, Span, SpanData, source_map::SourceMap};
use std::{ops::Range, sync::Arc};

// smoelius: Everything below this comment is based on:
// https://github.com/rust-lang/rust-clippy/blob/0b3c2ed81152a927c91c02d9d94762d2c65254a2/clippy_utils/src/source.rs#L16-L62

/// A type which can be converted to the range portion of a `Span`.
pub trait SpanRange {
    fn into_range(self) -> Range<BytePos>;
}
impl SpanRange for Span {
    fn into_range(self) -> Range<BytePos> {
        let data = self.data();
        data.lo..data.hi
    }
}
impl SpanRange for SpanData {
    fn into_range(self) -> Range<BytePos> {
        self.lo..self.hi
    }
}
impl SpanRange for Range<BytePos> {
    fn into_range(self) -> Range<BytePos> {
        self
    }
}

pub struct SourceFileRange {
    pub sf: Arc<SourceFile>,
    pub range: Range<usize>,
}
impl SourceFileRange {
    /// Attempts to get the text from the source file. This can fail if the source text isn't
    /// loaded.
    pub fn as_str(&self) -> Option<&str> {
        self.sf.src.as_ref().and_then(|x| x.get(self.range.clone()))
    }
}

/// Gets the source file, and range in the file, of the given span. Returns `None` if the span
/// extends through multiple files, or is malformed.
pub fn get_source_text(cx: &impl LintContext, sp: impl SpanRange) -> Option<SourceFileRange> {
    fn f(sm: &SourceMap, sp: Range<BytePos>) -> Option<SourceFileRange> {
        let start = sm.lookup_byte_offset(sp.start);
        let end = sm.lookup_byte_offset(sp.end);
        if !Arc::ptr_eq(&start.sf, &end.sf) || start.pos > end.pos {
            return None;
        }
        let range = start.pos.to_usize()..end.pos.to_usize();
        Some(SourceFileRange {
            sf: start.sf,
            range,
        })
    }
    f(cx.sess().source_map(), sp.into_range())
}
