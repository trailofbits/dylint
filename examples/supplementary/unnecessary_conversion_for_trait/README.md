# unnecessary_conversion_for_trait

### What it does
Checks for trait-behavior-preserving calls in positions where a trait implementation is
expected.

### Why is this bad?
Such unnecessary calls make the code more verbose and could impact performance.

### Example
```rust
let _ = Command::new("ls").args(["-a", "-l"].iter());
let _ = Path::new("/").join(Path::new("."));
```
Use instead:
```rust
let _ = Command::new("ls").args(["-a", "-l"]);
let _ = Path::new("/").join(".");
```
