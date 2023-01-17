# inconsistent_qualification

### What it does
Checks that a module's items are either imported or qualified with the module's path, but
not both.

### Why is this bad?
Mixing the two styles can lead to confusing code.

### Known problems
- No exception is made for for qualifications required for disambiguation.
- Re-exports may not be handled correctly.

### Example
```rust
use std::env::var;
fn main() {
    assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
}
```
Instead, either use:
```rust
use std::env::{var, VarError};
fn main() {
    assert_eq!(var("LD_PRELOAD"), Err(VarError::NotPresent));
}
```
Or use:
```rust
fn main() {
    assert_eq!(
        std::env::var("LD_PRELOAD"),
        Err(std::env::VarError::NotPresent)
    );
}
```
