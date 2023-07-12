# unnamed_constant

### What it does
Checks for unnamed constants, aka magic numbers.

### Why is this bad?
"Magic numbers are considered bad practice in programming, because they can make the code
more difficult to understand and harder to maintain." ([pandaquests])

### Example
```rust
x *= 1000;
```
Use instead:
```rust
const MILLIS: u64 = 1000;
x *= MILLIS;
```

### Configuration
- `threshold: u64` (default `1000`): Minimum value a constant must exceed to be flagged.

[pandaquests]: https://levelup.gitconnected.com/whats-so-bad-about-magic-numbers-4c0a0c524b7d
