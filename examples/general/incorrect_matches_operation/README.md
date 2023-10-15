# incorrect_matches_operation

### What it does
Checks for inefficient or incorrect use of the `matches!` macro.
Examples of inefficient or boiler plate uses:
- `matches!(obj, case1) | matches!(obj, case2)`
- `matches!(obj, case1) || matches!(obj, case2)`

Examples of incorrect uses (the condition is probably always false):
- `matches!(obj, case1) & matches!(obj, case2)`
- `matches!(obj, case1) && matches!(obj, case2)`

### Why is this bad?
One should use `matches!(obj, case1 | case2)` instead.

### Known problems
Since we use a pre-expansion-lint, we match the `matches!` argument tokens.
This is not ideal since we don't know if the argument is a variable name or, e.g.,
a call. If it is a call, this lint may result in a false positive, though I bet there won't
be many of those.


### Example
```rust
fn main() {
    let x = 1;
    if matches!(x, 123) | matches!(x, 256) {
        println!("Matches");
    }
}
```
Use instead:
```rust
fn main() {
    let x = 1;
    if matches!(x, 123 | 256) {
        println!("Matches");
    }
}
```
