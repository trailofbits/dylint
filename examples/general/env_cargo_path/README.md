# env_cargo_path

### What it does
Checks for `env!` or `option_env!` applied outside of a test to a Cargo environment variable
containing a path, e.g., `CARGO_MANIFEST_DIR`.

### Why is this bad?
The path might not exist when the code is used in production.

### Known problems
The lint does not apply inside macro arguments. So false negatives could result.

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
