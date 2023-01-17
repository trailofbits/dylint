# collapsible_unwrap

### What it does
Checks for an `unwrap` that could be combined with an `expect` or `unwrap` using `and_then`.

### Why is this bad?
Using `and_then`s tends to produce shorter method call chains, which are easier to read and
reason about.

### Known problems
The lint considers only `unwrap`s in method call chains. It does not consider unwrapped
values that are assigned to local variables, or assignments to local variables that are
later unwrapped, for example.

### Example
```rust,no_run
let package = toml.as_table().unwrap().get("package").unwrap();
```
Use instead:
```rust,no_run
let package = toml.as_table().and_then(|map| map.get("package")).unwrap();
```
