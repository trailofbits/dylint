# allow_clippy

**What it does:** This tongue-in-cheek lint checks for `#[allow(clippy::...)]`. It is
based on Clippy's `blanket_clippy_restriction_lints`:
https://rust-lang.github.io/rust-clippy/master/#blanket_clippy_restriction_lints

**Why is this bad?** It's not really. This is just an example of a Dylint library.

**Known problems:** None.

**Example:**
Bad:

```rust
#![allow(clippy::assertions_on_constants)]
```

Good:

```rust
#[deny(clippy::restriction, clippy::style, clippy::pedantic, clippy::complexity, clippy::perf, clippy::cargo, clippy::nursery)]
```

Returns the lint name if it is clippy lint.
