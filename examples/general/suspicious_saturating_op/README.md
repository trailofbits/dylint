# suspicious_saturating_op

### What it does
Checks for consecutive saturating operations.

### Why is this bad?
If the first operation saturates, the second operation may produce an incorrect result.

### Example
```rust
x = x.saturating_add(y).saturating_sub(z);
```
Use instead:
```rust
x = x.checked_add(y)?;
x = x.checked_sub(z)?;
```
