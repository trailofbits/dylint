# arg_iter

### What it does

Checks for functions that take `Iterator` trait bounds when they could use
`IntoIterator` instead.

### Why is this bad?

Using `IntoIterator` makes functions more flexible by allowing them to
accept more types like arrays, slices, and `Vec` without requiring explicit
`.iter()` calls. This often makes the API easier to use.

### Example

```rust
// Bad: Requires caller to call .iter() on Vec, slice, etc.
fn process_bad<I: Iterator<Item = u32>>(iter: I) {
    for item in iter {
        // ...
    }
}
```

Good: Accepts Vec, slice, etc. directly.

```rust
fn process_good<I: IntoIterator<Item = u32>>(iterable: I) {
    for item in iterable { // .into_iter() is implicitly called
        // ...
    }
}
```

This lint ignores cases where the parameter is also bounded by other traits
(besides the implicit `Sized`), as `IntoIterator` might not be suitable.

```rust
fn complex_bound<I: Iterator + Clone>(iter: I) { // Ok
    // ...
}
```
