use anstyle::{
    AnsiColor::{Red, Yellow},
    Style,
};
use std::io::{IsTerminal, Write};

// smoelius: `ColorizedError` is currently used only by `cargo-dylint`. But given the similarity of
// its implementation to `warn`, I prefer to keep it here for now. Also, FWIW, this limits the
// packages that directly depend on `anstyle`.

#[allow(clippy::module_name_repetitions)]
pub struct ColorizedError<E>(E)
where
    E: std::fmt::Debug;

impl<E> ColorizedError<E>
where
    E: std::fmt::Debug,
{
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

// smoelius: The use of `\r` is a bit of a hack, but it works, most notably with `anyhow`
// backtraces. Another way might be to implement the `Termination` trait, but that trait is still
// unstable: https://github.com/rust-lang/rust/issues/43301
impl<E> std::fmt::Debug for ColorizedError<E>
where
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let style = if std::io::stderr().is_terminal() {
            Style::new().fg_color(Some(Red.into())).bold()
        } else {
            Style::new()
        };
        write!(f, "\r{style}Error{style:#}: {:?}", self.0)
    }
}

pub type ColorizedResult<T> = Result<T, ColorizedError<anyhow::Error>>;

#[allow(clippy::expect_used)]
pub fn warn(opts: &crate::opts::Dylint, message: &str) {
    if !opts.quiet {
        let style = if std::io::stderr().is_terminal() {
            Style::new().fg_color(Some(Yellow.into())).bold()
        } else {
            Style::new()
        };
        // smoelius: Writing directly to `stderr` prevents capture by `libtest`.
        #[allow(clippy::panic)]
        writeln!(std::io::stderr(), "{style}Warning{style:#}: {message}")
            .unwrap_or_else(|error| panic!("Could not write to stderr: {error}"));
    }
}
