# suboptimal_pattern

### What it does
Checks for patterns that could perform additional destructuring.

### Why is this bad?
The use of destructuring patterns in closure parameters (for example) often leads to more
concise closure bodies. Beyond that, the benefits of this lint are similar to those of
[pattern-type-mismatch].

### Known problems
- Currently only checks closure parameters (not, e.g., match patterns).
- Currently only suggests destructuring references and tuples (not, e.g., arrays or
  structs).
- For the lint to suggest destructuring a reference, the idents involved must not use `ref`
  annotations.

### Example
```rust
let xs = [0, 1, 2];
let ys = xs.iter().map(|x| *x == 0).collect::<Vec<_>>();
```
Use instead:
```rust
let xs = [0, 1, 2];
let ys = xs.iter().map(|&x| x == 0).collect::<Vec<_>>();
```

### Configuration
- `explicit_deref_check: bool` (default `true`): By default, `suboptimal_pattern` will not
  suggest to destructure a reference unless it would eliminate at least one explicit
  dereference. Setting `explicit_deref_check` to `false` disables this check.

[pattern-type-mismatch]: https://rust-lang.github.io/rust-clippy/master/#pattern_type_mismatch
