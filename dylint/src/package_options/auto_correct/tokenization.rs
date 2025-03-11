use anyhow::Result;
use std::sync::LazyLock;
use syntect::parsing::{ParseState, ScopeStackOp, SyntaxReference, SyntaxSet};

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_nonewlines);

static SYNTAX: LazyLock<&SyntaxReference> =
    LazyLock::new(|| SYNTAX_SET.find_syntax_by_extension("rs").unwrap());

pub fn tokenize_lines<T: AsRef<str>>(lines: &[T]) -> Result<Vec<&str>> {
    let tokens = lines
        .iter()
        .map(|line| -> Result<_> { tokenize_fragment(line.as_ref().trim()) })
        .collect::<Result<Vec<Vec<_>>>>()?;

    Ok(tokens.into_iter().flatten().collect())
}

/// Tokenize a Rust fragment, e.g., code containing unbalanced delimiters.
pub fn tokenize_fragment(line: &str) -> Result<Vec<&str>> {
    let mut state = ParseState::new(*SYNTAX);
    let mut offsets_and_ops = state.parse_line(line, &SYNTAX_SET)?;
    offsets_and_ops.push((line.len(), ScopeStackOp::Noop));
    let tokens = offsets_and_ops
        .windows(2)
        .filter_map(|w| {
            let &[(start, _), (end, _)] = w else {
                unreachable!();
            };
            let token = &line[start..end];
            if token.chars().all(char::is_whitespace) {
                return None;
            }
            // smoelius: Several of the "tokens" that `syntect` identifies begin with whitespace,
            // hence the use of `trim_start`.
            Some(token.trim_start())
        })
        .collect();
    Ok(tokens)
}
