# ref_aware_redundant_closure_for_method_calls

### What it does
This is essentially a ref-aware fork of Clippy's [`redundant_closure_for_method_calls`]
lint. It suggests to remove a closure when made possible by a use of `as_ref`, `as_mut`,
`as_deref`, or `as_deref_mut`.

### Known problems
Currently works only for [`Option`]s.

### Example
```rust
Some(String::from("a")).map(|s| s.is_empty());
Some(String::from("a")).map(|s| s.to_uppercase());
```
Use instead:
```rust
Some(String::from("a")).as_ref().map(String::is_empty);
Some(String::from("a")).as_deref().map(str::to_uppercase);
```

[`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
[`redundant_closure_for_method_calls`]: https://rust-lang.github.io/rust-clippy/master/#redundant_closure_for_method_calls
