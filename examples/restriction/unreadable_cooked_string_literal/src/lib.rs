#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;

use clippy_utils::{diagnostics::span_lint_and_sugg, source::snippet_opt};
use regex::Regex;
use rustc_ast::ast::{LitKind, StrStyle};
use rustc_errors::Applicability;
use rustc_hir::{HirId, Lit};
use rustc_lint::{LateContext, LateLintPass};
use std::sync::LazyLock;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for cooked string literals that would be more readable as raw string literals.
    /// Specifically, the lint checks for a cooked string literal that:
    ///
    /// - contains '\n' or '\"' not at the beginning or end of the literal
    /// - contains no escaped characters besides '\n', '\"', or '\\'
    ///
    /// ### Why is this bad?
    ///
    /// Such literals are more readable as raw string literals.
    ///
    /// ### Example
    ///
    /// ```rust
    /// println!("fn main() {{\n    println!(\"Hello, world!\");\n}}");
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// println!(r#"fn main() {{
    ///     println!("Hello, world!");
    /// }}"#);
    /// ```
    pub UNREADABLE_COOKED_STRING_LITERAL,
    Warn,
    "cooked string literals that would be more readable as raw string literals"
}

#[cfg_attr(
    dylint_lib = "unreadable_cooked_string_literal",
    allow(unreadable_cooked_string_literal)
)]
#[test]
fn sanity() {
    assert_eq!(
        "fn main() {{\n    println!(\"Hello, world!\");\n}}",
        r#"fn main() {{
    println!("Hello, world!");
}}"#
    );
}

#[cfg_attr(dylint_lib = "supplementary", allow(commented_out_code))]
impl<'tcx> LateLintPass<'tcx> for UnreadableCookedStringLiteral {
    fn check_lit(
        &mut self,
        cx: &LateContext<'tcx>,
        _hir_id: HirId,
        lit: Lit,
        _is_negated_pat: bool,
    ) {
        if let LitKind::Str(_symbol, StrStyle::Cooked) = lit.node
            // && let _ = dbg!(_symbol.as_str())
            // smoelius: `symbol.as_str()` gives us an unescaped string literal. But we need the
            // escaped string literal to check that it contains contains no escaped characters
            // besides '\n', '\"', or '\\'.
            && let Some(s) = snippet_opt(cx, lit.span)
            // smoelius: Strip quotes.
            && 2 <= s.len()
            && s.as_bytes()[0] == b'\"'
            && s.as_bytes()[s.len() - 1] == b'\"'
            && let s = &s[1..s.len() - 1]
            // smoelius: 2 because an escaped character consists of two bytes.
            && 2 <= s.len()
            && let chars = escaped_chars(s).collect::<Vec<_>>()
            && chars
                .iter()
                .any(|&(i, x)| b"n\"".contains(&x) && i != 0 && i != s.len() - 2)
            && chars
                .iter()
                .all(|&(_, x)| b"n\"\\".contains(&x))
        {
            // smoelius: `symbol.as_str()` gives us an unescaped string literal, but the `{{` and
            // `}}` are also unescaped. We need to unescape the backslashed characters, but keep the
            // `{{` and `}}` escaped.
            let s_unescaped = unescape(s, chars);
            let n_hashes = required_hashes(&s_unescaped);
            span_lint_and_sugg(
                cx,
                UNREADABLE_COOKED_STRING_LITERAL,
                lit.span,
                r"cooked string literal that would be more readable raw",
                r"use",
                format!(r#"r{:#>n_hashes$}"{s_unescaped}"{:#>n_hashes$}"#, "", ""),
                Applicability::MachineApplicable,
            );
        }
    }
}

fn escaped_chars(s: &str) -> impl Iterator<Item = (usize, u8)> {
    s.as_bytes().windows(2).enumerate().filter_map(|(i, w)| {
        let [fst, snd] = w.try_into().unwrap();
        if fst == b'\\' { Some((i, snd)) } else { None }
    })
}

fn unescape(s: &str, iter: impl IntoIterator<Item = (usize, u8)>) -> String {
    let mut buf = String::new();
    let mut end = 0;
    for (i, x) in iter {
        buf.push_str(&s[end..i]);
        buf.push(match x {
            b'n' => '\n',
            b'"' => '\"',
            b'\\' => '\\',
            _ => panic!("unexpected char: {:?}", x as char),
        });
        end = i + 2;
    }
    buf.push_str(&s[end..]);
    buf
}

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r##""#*|#*""##).unwrap());

fn required_hashes(s_unescaped: &str) -> usize {
    #[allow(clippy::len_zero)]
    RE.captures_iter(s_unescaped)
        .map(|caps| {
            assert_eq!(1, caps.len());
            let cap = &caps[0];
            // smoelius: 1 for the quote. Note that the full length of the capture is returned,
            // which is one more than the number of hashes. The maximum of all such values is the
            // required hashes.
            assert!(1 <= cap.len());
            cap.len()
        })
        .max()
        .unwrap_or_default()
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
