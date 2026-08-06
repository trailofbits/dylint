#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint, macros::root_macro_call_first_node, source::snippet_opt,
};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::sym;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks for `format!(...)` invocations with `env!(...)` arguments and string literal
    /// fragments where `concat!(...)` could be used instead.
    ///
    /// ### Why is this bad?
    ///
    /// When a `format!()` invocation combines only string literal and `env!(...)` arguments with
    /// default `{}` formatting, `concat!()` can perform the concatenation at compile time instead
    /// of at runtime.
    ///
    /// ### Known problems
    ///
    /// The lint currently recognizes only `env!(...)` and cooked string literals, and emits only
    /// when at least one argument is an `env!(...)` invocation. `format!()` invocations involving
    /// raw strings and other compile-time constants are not checked.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # let _ =
    /// format!("{}/**/*.md", env!("CARGO_MANIFEST_DIR"))
    /// # ;
    /// ```
    ///
    /// This can be written as:
    ///
    /// ```rust
    /// # let _ =
    /// concat!(env!("CARGO_MANIFEST_DIR"), "/**/*.md")
    /// # ;
    /// ```
    pub CONCATENABLE_FORMAT_ARGS,
    Warn,
    "`format!(...)` invocations where `concat!(...)` could be used instead"
}

impl<'tcx> LateLintPass<'tcx> for ConcatenableFormatArgs {
    /// Check each expression for a lintable `format!` invocation. `root_macro_call_first_node`
    /// fires only on the HIR node that introduces the macro expansion, so each `format!` call is
    /// processed exactly once without explicit deduplication.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else {
            return;
        };

        // Verify this is `std::format!` specifically (not `println!`, `write!`, etc.).
        if !cx
            .tcx
            .is_diagnostic_item(sym::format_macro, macro_call.def_id)
        {
            return;
        }

        let format_span = macro_call.span;

        // Get the source snippet of the `format!` invocation.
        let Some(snippet) = snippet_opt(cx, format_span) else {
            return;
        };

        // Check source instead of HIR because `format!` expansion does not preserve all
        // user arguments in one walkable tree (`format_args` lowering hides arg literals).
        if !is_concatenable_format_args(&snippet) {
            return;
        }

        span_lint(
            cx,
            CONCATENABLE_FORMAT_ARGS,
            format_span,
            "this `format!()` invocation could be replaced with `concat!()`",
        );
    }
}

/// Return whether a `format!` invocation has only `concat!()`-compatible arguments (cooked string
/// literals and `env!(...)` invocations) under default `{}` formatting, with at least one
/// `env!(...)` argument present.
fn is_concatenable_format_args(snippet: &str) -> bool {
    let Some((_, content, rest)) = parse_macro_invocation(snippet) else {
        return false;
    };
    if !rest.trim_start().is_empty() {
        return false;
    }

    // Parse the format string.
    let s = content.trim_start();
    let Some(mut s) = parse_format_string(s) else {
        return false;
    };

    // Collect arguments that can be passed to `concat!()`.
    let mut has_env_macro = false;
    loop {
        s = s.trim_start();

        // Check for end or comma.
        if s.is_empty() {
            break;
        }
        if !s.starts_with(',') {
            return false;
        }
        s = s[1..].trim_start();

        // Check if there's another argument.
        if s.is_empty() {
            break;
        }

        let Some((is_env_macro, rest)) = extract_concat_arg(s) else {
            return false;
        };
        has_env_macro |= is_env_macro;
        s = rest;
    }

    // Require at least one `env!(...)` arg. A `format!` of only string literals (e.g.,
    // `format!("a", "b")`) could also be rewritten with `concat!()`. But that pattern is unusual
    // and is left to other lints.
    has_env_macro
}

/// Extract a supported argument (a cooked string literal or an `env!(...)`-named macro invocation,
/// including qualified paths like `std::env!`) and return whether it is the macro form along with
/// the remaining input.
fn extract_concat_arg(s: &str) -> Option<(bool, &str)> {
    if let Some((name, _, rest)) = parse_macro_invocation(s)
        && is_env_macro(name)
    {
        return Some((true, rest));
    }

    if let Some(rest) = skip_string(s) {
        return Some((false, rest));
    }

    None
}

/// Parse a macro invocation and return its name, arguments, and remaining input.
fn parse_macro_invocation(snippet: &str) -> Option<(&str, &str, &str)> {
    let snippet = snippet.trim_start();
    let bang = snippet.find('!')?;
    let name = snippet[..bang].trim_end();
    let (arguments, rest) = parse_macro_arguments(&snippet[bang..])?;
    Some((name, arguments, rest))
}

/// Parse a format string and reject non-simple placeholders, returning the remaining input.
fn parse_format_string(s: &str) -> Option<&str> {
    let s = s.strip_prefix('"')?;
    let mut char_indices = s.char_indices();

    while let Some((i, c)) = char_indices.next() {
        if c == '"' {
            return Some(&s[i + 1..]);
        }

        if c == '\\' {
            char_indices.next()?;
            continue;
        }

        if c == '{' {
            let (_, next) = char_indices.next()?;
            match next {
                '{' => {}
                '}' => {}
                _ => return None,
            }
            continue;
        }

        if c == '}' {
            let (_, next) = char_indices.next()?;
            if next != '}' {
                return None;
            }
            continue;
        }
    }

    None
}

/// Return whether `name` is a syntactic reference to the `env!` macro, including qualified paths
/// like `std::env!` and `core::env!`.
fn is_env_macro(name: &str) -> bool {
    matches!(
        name,
        "env" | "std::env" | "core::env" | "::std::env" | "::core::env"
    )
}

/// Parse macro arguments and return the text inside the parentheses plus the remaining input.
fn parse_macro_arguments(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let s = s.strip_prefix('!')?;
    let s = s.trim_start();
    let s = s.strip_prefix('(')?;
    let closing_paren_offset = closing_paren_offset(s)?;
    Some((&s[..closing_paren_offset], &s[closing_paren_offset + 1..]))
}

/// Find the closing parenthesis matching an already-consumed opening parenthesis.
fn closing_paren_offset(s: &str) -> Option<usize> {
    let mut depth = 1_usize;
    let mut char_indices = s.char_indices();

    while let Some((i, ch)) = char_indices.next() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            '"' => {
                skip_string_chars(&mut char_indices)?;
            }
            _ => {}
        }
    }

    None
}

/// Skip a string literal and return the remaining input.
fn skip_string(s: &str) -> Option<&str> {
    let s = s.strip_prefix('"')?;
    let mut char_indices = s.char_indices();
    let close = skip_string_chars(&mut char_indices)?;
    Some(&s[close + 1..])
}

/// Skip string characters and return the closing quote offset.
fn skip_string_chars(char_indices: &mut std::str::CharIndices<'_>) -> Option<usize> {
    while let Some((i, ch)) = char_indices.next() {
        match ch {
            '\\' => {
                char_indices.next()?;
            }
            '"' => return Some(i),
            _ => {}
        }
    }

    None
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
