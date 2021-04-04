use std::env;
use std::ffi::OsString;

pub fn main() -> dylint::ColorizedResult<()> {
    env_logger::init();

    let args: Vec<_> = env::args().map(OsString::from).collect();

    dylint::cargo_dylint(&args)
}
