# non_topologically_sorted_functions

### What it does

It enforces a relative order among functions defined within a module. Callers must precede
their module-local callees. Functions called by the same caller should appear in call order,
unless that ordering would conflict with caller-callee constraints. Caller-callee
constraints take precedence, and a call-order constraint is ignored if adding it would
create a cycle.

### Why is this bad?

Without a certain order, it can be difficult to navigate through the module's functions.

### Example

```rust
fn bar() { }

fn foo() {
    bar();
}
```

Use instead:

```rust
fn foo() {
    bar();
}

fn bar() { }
```
