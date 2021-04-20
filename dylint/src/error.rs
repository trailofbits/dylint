use ansi_term::Color::{Red, Yellow};

#[allow(clippy::module_name_repetitions)]
pub struct ColorizedError<E>(E)
where
    E: std::fmt::Debug;

impl<E> ColorizedError<E>
where
    E: std::fmt::Debug,
{
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
        write!(f, "\r{}: {:?}", Red.bold().paint("Error"), self.0)
    }
}

pub type ColorizedResult<T> = Result<T, ColorizedError<anyhow::Error>>;

pub fn warn(opts: &crate::Dylint, message: &str) {
    if !opts.quiet {
        eprintln!("{}: {}", Yellow.bold().paint("Warning"), message);
    }
}
