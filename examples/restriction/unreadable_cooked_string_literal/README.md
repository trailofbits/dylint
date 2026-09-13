# unreadable_cooked_string_literal

### What it does

Checks for cooked string literals that would be more readable as raw string literals.
Specifically, the lint checks for a cooked string literal that:

- contains '\n' or '\"' not at the beginning or end of the literal
- contains no escaped characters other than '\n', '\"', or '\\'

### Why is this bad?

Such literals are more readable as raw string literals.

### Example

```rust
println!("fn main() {{\n    println!(\"Hello, world!\");\n}}");
```

Use instead:

```rust
println!(r#"fn main() {{
    println!("Hello, world!");
}}"#);
```
