#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_ast::{
    BinOpKind, Expr, ExprKind, MacCall, token::Token, token::TokenKind, tokenstream::TokenTree,
};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Symbol;

dylint_linting::declare_pre_expansion_lint! {
    /// ### What it does
    ///
    /// Checks for inefficient or incorrect use of the `matches!` macro.
    /// Examples of inefficient or boiler plate uses:
    ///
    /// - `matches!(obj, case1) | matches!(obj, case2)`
    /// - `matches!(obj, case1) || matches!(obj, case2)`
    ///
    /// Examples of incorrect uses (the condition is probably always false):
    ///
    /// - `matches!(obj, case1) & matches!(obj, case2)`
    /// - `matches!(obj, case1) && matches!(obj, case2)`
    ///
    /// ### Why is this bad?
    ///
    /// One should use `matches!(obj, case1 | case2)` instead.
    ///
    /// ### Known problems
    ///
    /// Since we use a pre-expansion-lint, we match the `matches!` argument tokens.
    /// This is not ideal since we don't know if the argument is a variable name or, e.g.,
    /// a call. If it is a call, this lint may result in a false positive, though I bet there won't
    /// be many of those.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn main() {
    ///     let x = 1;
    ///     if matches!(x, 123) | matches!(x, 256) {
    ///         println!("Matches");
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn main() {
    ///     let x = 1;
    ///     if matches!(x, 123 | 256) {
    ///         println!("Matches");
    ///     }
    /// }
    /// ```
    pub INCORRECT_MATCHES_OPERATION,
    Warn,
    "inefficient `matches!` macro use"
}

fn is_matches_macro(expr: &Expr) -> Option<&MacCall> {
    if let ExprKind::MacCall(mac) = &expr.kind // must be a macro call
        && mac.path == Symbol::intern("matches")
    // must be a matches! symbol
    {
        return Some(mac);
    }
    None
}

/// Finds out if first arguments of two macro calls matches This is done by finding all tokens that
/// belong to first argument, and comparing them without considering the span information (since
/// span info would always differ for the two different macro calls arguments)
fn macro_call_first_arg_equals(m1: &MacCall, m2: &MacCall) -> bool {
    let t1 = m1.args.tokens.iter();
    let t2 = m2.args.tokens.iter();

    for (t1, t2) in std::iter::zip(t1, t2) {
        // If any of the tokens are a comma, that means we are past the first argument
        // on one of the macros. At this point, we must check if both args have the same number
        // of tokens (because we verified the match of those tokens already)
        let c1 = is_comma_token(t1);
        let c2 = is_comma_token(t2);
        if c1 || c2 {
            return c1 && c2;
        }
        // If neither token is a comma, make sure they match: if they don't return false!
        else if !t1.eq_unspanned(t2) {
            return false;
        }
    }
    unreachable!("This should never happen: matches! macro did not have a comma token?");
}

/// Returns whether a given token is a comma
const fn is_comma_token(tree: &TokenTree) -> bool {
    matches!(
        tree,
        TokenTree::Token(
            Token {
                kind: TokenKind::Comma,
                ..
            },
            _
        )
    )
}

impl EarlyLintPass for IncorrectMatchesOperation {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if let ExprKind::Binary(op, left, right) = &expr.kind
            // Look for binary operators
            // Ensure the binary operator is |, ||, &&, &
            && matches!(
                op.node,
                BinOpKind::BitOr | BinOpKind::Or | BinOpKind::And | BinOpKind::BitAnd
            )
            // The left side needs to be a matches! macro call
            && let Some(matches1) = is_matches_macro(left)
            // The right side needs to be a matches! macro call
            && let Some(matches2) = is_matches_macro(right)
            && macro_call_first_arg_equals(matches1, matches2)
        // a MacCall structure has path and arguments, the arguments are tokens
        // the tokens are e.g. an Ident with variable name or a comma
        // so we need to fetch the first argument of the macro call by processing those tokens
        {
            match op.node {
                // For | and || operator, just say it can be rewritten
                BinOpKind::BitOr | BinOpKind::Or => {
                    span_lint(
                        cx,
                        INCORRECT_MATCHES_OPERATION,
                        expr.span,
                        "This matches! macro use can be rewritten to matches!(obj, A | B)",
                    );
                }
                // For & and && operators, this is likely a bug
                BinOpKind::BitAnd | BinOpKind::And => {
                    let op = if op.node == BinOpKind::BitAnd {
                        "&"
                    } else {
                        "&&"
                    };
                    let msg = format!(
                        "Is this a bug? matches!(obj, A) {op} matches!(obj, B) is (almost) always false"
                    );
                    span_lint(cx, INCORRECT_MATCHES_OPERATION, expr.span, msg);
                }
                _ => {
                    unreachable!("This should never happen - op.node can't be other operator");
                }
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
