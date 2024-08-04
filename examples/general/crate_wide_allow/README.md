# crate_wide_allow

### What it does
Checks for use of `#![allow(...)]` at the crate level.

### Why is this bad?
Such uses cannot be overridden with `--warn` or `--deny` from the command line. They _can_
be overridden with `--force-warn` or `--forbid`, but one must know the `#![allow(...)]`
are present to use these unconventional options.

### Example
```rust
#![allow(clippy::assertions_on_constants)] // in code
```
Use instead:
```rust
// Allow `clippy::assertions-on-constants` in Cargo.toml. See:
// - https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
// - https://doc.rust-lang.org/clippy/configuration.html#lints-section-in-cargotoml
```
