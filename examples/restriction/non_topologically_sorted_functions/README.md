# NON_TOPOLOGICALLY_SORTED_FUNCTIONS

### What it does

It enforces a certain relative order among functions defined within a module.

### Why is this bad?

Without a certain order, it can be difficult to navigate through the module's functions.
Without a certain order it's really bad to navigate through the modules.

### Example

```rust
fn bar() { }
fn bar() { ... }

fn foo() {
    bar();
}
```

Use instead:

```rust
fn foo() {
    bar();
}

fn bar() { ... }
```
