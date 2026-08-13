# non_topologically_sorted_functions

### What it does

It enforces a relative order among functions defined within a module. Callers must precede
their module-local callees. Functions called by the same caller should appear in call order.
A constraint is rejected if it would create a cycle when combined with previously accepted
constraints.

### Why is this bad?

Without a certain order, it can be difficult to navigate through the module's functions.

### Example

```rust
fn bar() {}

fn foo() {
    bar();
}
```

Use instead:

```rust
fn foo() {
    bar();
}

fn bar() {}
```

### Known problems

While the lint may seem strict, its rules do not completely dictate the order of a module's
functions. Judgement must often be exercised to address the lint's warnings.
