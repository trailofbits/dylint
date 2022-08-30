use ansi_term::{
    Color::{Red, Yellow},
    Style,
};
use std::io::Write;

// smoelius: `ColorizedError` is currently used only by `cargo-dylint`. But given the similarity of
// its implementation to `warn`, I prefer to keep it here for now. Also, FWIW, this limits the
// packages that directly depend on `ansi_term`.

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

// smoelius: The use of `\r` is a bit of a hack, but it works, most notably with `anyhow` backtraces
// (which require a nightly compiler, BTW). Another way might be to implement the `Termination`
// trait, but that trait is still unstable: https://github.com/rust-lang/rust/issues/43301
impl<E> std::fmt::Debug for ColorizedError<E>
where
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{:?}",
            if atty::is(atty::Stream::Stdout) {
                format!("\r{}: ", Red.bold().paint("Error"))
            } else {
                String::new()
            },
            self.0
        )
    }
}

pub type ColorizedResult<T> = Result<T, ColorizedError<anyhow::Error>>;

#[allow(clippy::expect_used)]
pub fn warn(opts: &crate::Dylint, message: &str) {
    if !opts.quiet {
        // smoelius: Writing directly to `stderr` avoids capture by `libtest`.
        std::io::stderr()
            .write_fmt(format_args!(
                "{}: {}\n",
                if atty::is(atty::Stream::Stdout) {
                    Yellow.bold()
                } else {
                    Style::new()
                }
                .paint("Warning"),
                message
            ))
            .expect("Could not write to stderr");
    }
}
