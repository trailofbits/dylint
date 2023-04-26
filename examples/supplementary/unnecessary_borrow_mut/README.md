# unnecessary_borrow_mut

### What it does
Checks for calls to [`RefCell::borrow_mut`] that could be calls to [`RefCell::borrow`].

### Why is this bad?
A call to [`RefCell::borrow_mut`] "panics if the value is currently borrowed." Thus, a call
to [`RefCell::borrow_mut`] can panic in situations where a call to [`RefCell::borrow`] would
not.

### Example
```rust
x = *cell.borrow_mut();
```
Use instead:
```rust
x = *cell.borrow();
```

[`RefCell::borrow_mut`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow_mut
[`RefCell::borrow`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow
