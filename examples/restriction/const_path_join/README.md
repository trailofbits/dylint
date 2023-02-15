# const_path_join

### What it does
Checks for joining of constant path components.

### Why is this bad?
Such paths can be constructed from string literals using `/`, since `/` works as a path
separator on both Unix and Windows (see [`std::path::Path`]).

### Example
```rust
PathBuf::from("..").join("target")
```
Use instead:
```rust
PathBuf::from("../target")
```

[`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
