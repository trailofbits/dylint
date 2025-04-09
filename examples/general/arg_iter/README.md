# arg_iter

### What it does
Checks for functions that take `Iterator` trait bounds when they could use
`IntoIterator` instead.

### Why is this bad?
Using `IntoIterator` makes functions more flexible by allowing them to
accept more types like arrays, slices, and Vec without requiring explicit 
`.iter()` calls.

### Example
```rust
// Bad
fn process_bad<I: Iterator<Item = u32>>(iter: I) {
    // ...
}

// Good
fn process_good<I: IntoIterator<Item = u32>>(iter: I) {
    // ...
}
```
