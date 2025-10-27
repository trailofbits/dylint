# inconsistent_struct_pattern

### What it does

Checks for struct patterns whose fields whose fields do not match their declared order.

### Why is this bad?

It can be harder to spot mistakes in inconsistent code.

### Example

```rust
struct Struct {
    a: bool,
    b: bool,
};
let strukt = Struct { a: false, b: true };
let Struct { b, a } = strukt;
```

Use instead:

```rust
struct Struct {
    a: bool,
    b: bool,
};
let strukt = Struct { a: false, b: true };
let Struct { a, b } = strukt;
```
