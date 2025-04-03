# abs_home_path

### What it does

Checks for string literals that are absolute paths into the user's home directory, e.g.,
`env!("CARGO_MANIFEST_DIR")`.

### Why is this bad?

The path might not exist when the code is used in production.

### Known problems

The lint does not apply inside macro arguments. So false negatives could result.

### Note

This lint doesn't warn in build scripts (`build.rs`), as they often need to reference absolute paths.

### Example

```rust
fn main() {
    let path = option_env!("CARGO");
    println!("{:?}", path);
}
```

Use instead:

```rust
fn main() {
    let path = std::env::var("CARGO");
    println!("{:?}", path);
}
```
