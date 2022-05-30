# crate_wide_allow

**What it does:** Checks for use of `#![allow(...)]` at the crate level.

**Why is this bad?** Such uses cannot be overridden with `--warn` or `--deny` from the
command line. They _can_ be overridden with `--force-warn` or `--forbid`, but one must
know the `#![allow(...)]` are present to use these unconventional options.

**Known problems:** None.

**Example:**
Bad:

```rust
#![allow(clippy::assertions_on_constants)] // in code
```

Good:

```rust
// Pass `--allow clippy::assertions-on-constants` on the command line.
```
