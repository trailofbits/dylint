# local_ref_cell

### What it does

Checks for local variables that are [`RefCell`]s.

### Why is this bad?

There is rarely a need for a locally declared `RefCell`.

### Example

```rust
let x = RefCell::<usize>::new(0);
```

Use instead:

```rust
let mut x: usize = 0;
```

[`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
