use dylint_internal::env::{self, enabled};
use std::ffi::OsString;

pub fn main() -> dylint::ColorizedResult<()> {
    env_logger::init();

    let args: Vec<_> = std::env::args().map(OsString::from).collect();

    let result = dylint::cargo_dylint(&args);

    if result.is_err() && enabled(env::RUST_BACKTRACE) {
        eprintln!(
            "If you don't see a backtrace below, it could be because `cargo-dylint` wasn't built \
            with a nightly compiler."
        );
    }

    result
}
