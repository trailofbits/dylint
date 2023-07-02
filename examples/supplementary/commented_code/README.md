# commented_code

### What it does
Checks for code that has been commented out.

### Why is this bad?
Commented code is often meant to be removed, but kept by mistake.

### Known problems
- Currently only checks for commented out statements in blocks.
- Does not handle statements spanning multiple line comments, e.g.:

  ```rust
  // dbg!(
  //   x
  // );
  ```

### Example
```rust
// dbg!(x);
f(x);
```
Use instead:
```rust
f(x);
```
