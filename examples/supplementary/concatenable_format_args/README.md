# concatenable_format_args

### What it does

Checks for `format!(...)` invocations with `env!(...)` arguments and string literal
fragments where `concat!(...)` could be used instead.

### Why is this bad?

When a `format!()` invocation combines only string literal and `env!(...)` arguments with
default `{}` formatting, `concat!()` can perform the concatenation at compile time instead
of at runtime.

### Known problems

The lint currently recognizes only `env!(...)` and cooked string literals, and emits only
when at least one argument is an `env!(...)` invocation. `format!()` invocations involving
raw strings and other compile-time constants are not checked.

### Example

```rust
format!("{}/**/*.md", env!("CARGO_MANIFEST_DIR"))
```

This can be written as:

```rust
concat!(env!("CARGO_MANIFEST_DIR"), "/**/*.md")
```
