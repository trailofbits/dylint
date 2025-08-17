#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint_and_help, source::SpanRangeExt};
use regex::{Captures, Regex};
use rustc_hir::Block;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::{BytePos, Span};
use std::sync::LazyLock;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for code that has been commented out.
    ///
    /// ### Why is this bad?
    ///
    /// Commented-out code is often meant to be removed, but kept by mistake.
    ///
    /// ### Known problems
    ///
    /// - Currently only checks for commented-out statements in blocks.
    /// - Does not handle statements spanning multiple line comments, e.g.:
    ///
    ///   ```rust
    ///   // dbg!(
    ///   //   x
    ///   // );
    ///   ```
    ///
    /// ### Example
    ///
    /// ```rust
    /// # fn f(_: u32) {}
    /// # let x = 0;
    /// // dbg!(x);
    /// f(x);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # fn f(_: u32) {}
    /// # let x = 0;
    /// f(x);
    /// ```
    pub COMMENTED_OUT_CODE,
    Warn,
    "code that has been commented out"
}

impl<'tcx> LateLintPass<'tcx> for CommentedOutCode {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if block.stmts.is_empty() {
            check_span(
                cx,
                block
                    .span
                    .with_lo(block.span.lo() + BytePos(1))
                    .with_hi(block.span.hi() - BytePos(1)),
            );
        } else {
            // smoelius: Ensure that the block's span and its first statement's span are from the
            // same context. They can differ if one of them was produced by a macro, for example.
            // This problem was observe in the `cargo-unmaintained` codebase. I have not yet found a
            // small example to reproduce the problem.
            if block.span.ctxt() == block.stmts.first().unwrap().span.ctxt() {
                check_span(
                    cx,
                    block
                        .span
                        .with_lo(block.span.lo() + BytePos(1))
                        .with_hi(block.stmts.first().unwrap().span.lo()),
                );
            }
            for window in block.stmts.windows(2) {
                check_span(
                    cx,
                    block
                        .span
                        .with_lo(window[0].span.hi())
                        .with_hi(window[1].span.lo()),
                );
            }
            if block.stmts.last().unwrap().span.ctxt() == block.span.ctxt() {
                check_span(
                    cx,
                    block
                        .span
                        .with_lo(block.stmts.last().unwrap().span.hi())
                        .with_hi(block.span.hi() - BytePos(1)),
                );
            }
        }
    }
}

static LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(^|[^/])(//([^/].*))").unwrap());
static BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/\*(([^*]|\*[^/])*)\*/").unwrap());

fn check_span(cx: &LateContext<'_>, span: Span) {
    if span.from_expansion() {
        return;
    }
    let Some(source_file_range) = span.get_source_text(cx) else {
        return;
    };
    let text = source_file_range.as_str();
    for captures in LINE_COMMENT.captures_iter(text) {
        assert_eq!(4, captures.len());
        check_captures(cx, span, &captures, 2, 3);
    }
    for captures in BLOCK_COMMENT.captures_iter(text) {
        assert_eq!(3, captures.len());
        check_captures(cx, span, &captures, 0, 1);
    }
}

fn check_captures(
    cx: &LateContext<'_>,
    span: Span,
    captures: &Captures,
    span_index: usize,
    text_index: usize,
) {
    let range = captures.get(span_index).unwrap().range();
    let text = &captures[text_index];

    // smoelius: `needless_doctest_main` provides an example use of `rustc_parse`, should we decide
    // to use that rather than `syn`:
    // https://github.com/rust-lang/rust-clippy/blob/e8d02fe1cb20a21e0bee2f80f1e16945eb3b9437/clippy_lints/src/doc/needless_doctest_main.rs#L45-L59
    let Ok(block) = syn::parse_str::<syn::Block>(&format!("{{{text}}}")) else {
        return;
    };

    if block.stmts.is_empty() {
        return;
    }

    if let [syn::Stmt::Expr(syn::Expr::Path(expr_path), None)] = block.stmts.as_slice()
        && expr_path_is_ident(expr_path)
    {
        return;
    }

    #[expect(clippy::cast_possible_truncation)]
    span_lint_and_help(
        cx,
        COMMENTED_OUT_CODE,
        span.with_lo(span.lo() + BytePos(range.start as u32))
            .with_hi(span.lo() + BytePos(range.end as u32)),
        "commented-out code",
        None,
        "uncomment or remove",
    );
}

fn expr_path_is_ident(expr_path: &syn::ExprPath) -> bool {
    let syn::ExprPath { attrs, qself, path } = expr_path;
    attrs.is_empty() && qself.is_none() && path.get_ident().is_some()
}

#[test]
fn ui() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .rustc_flags(["--edition=2024"])
        .run();
}
