// smoelius: `proc_macro_hygiene` appears to be unnecessary since:
// https://github.com/rust-lang/rust/pull/157857
#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![allow(unused_features)]
#![feature(proc_macro_hygiene)]

// smoelius: rust-lang/rust-clippy#14705 was merged on 2025-05-05 and Clippy's toolchain was
// subsequently updated to nightly-2025-05-14.
#[rustversion::before(2025-05-14)]
fn main() {}

#[rustversion::since(2025-05-14)]
mod extra_symbols;

#[rustversion::since(2025-05-14)]
fn main() -> anyhow::Result<()> {
    extra_symbols::build()
}
