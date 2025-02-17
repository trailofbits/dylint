# escaping_doc_link

### What it does

Checks for doc comment links that refer to files outside of their source file's package.

### Why is this bad?

Such links will be broken on [docs.rs], for example.

### Example

```rust
//! [general-purpose lints]: ../../general
```

Use instead:

```rust
//! [general-purpose lints]: https://github.com/trailofbits/dylint/tree/master/examples/general
```

[docs.rs]: https://docs.rs
