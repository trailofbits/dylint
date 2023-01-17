# env_literal

### What it does
Checks for environment variables referred to with string literals.

### Why is this bad?
A typo in the string literal will result in a runtime error, not a compile time error.

### Example
```rust
let _ = std::env::var("RUSTFLAGS");
std::env::remove_var("RUSTFALGS"); // Oops
```
Use instead:
```rust
const RUSTFLAGS: &str = "RUSTFLAGS";
let _ = std::env::var(RUSTFLAGS);
std::env::remove_var(RUSTFLAGS);
```
