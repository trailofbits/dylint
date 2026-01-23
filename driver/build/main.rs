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
