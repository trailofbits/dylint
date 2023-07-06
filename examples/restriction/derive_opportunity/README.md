# derive_opportunity

### What it does
Checks for data structures that could derive additional traits.

### Why is this bad?
Not deriving the additional traits could be a missed opportunity.

### Known problems
- This lint is noisy! The `at_least_one_field` and `ignore` options (see below) can be used
  to make the lint less noisy.
- Currently does not support traits with type or constant parameters (e.g., `PartialEq`), or
  traits with supertraits with type or constant parameters (e.g., `Eq`).

### Example
```rust
#[derive(Default)]
struct S;

struct T(S);
```
Use instead:
```rust
#[derive(Default)]
struct S;

#[derive(Default)]
struct T(S);
```

### Configuration
- `at_least_one_field: bool` (default `false`): If set to `true`, the lint suggests to
  derive a trait only when there is at least one field that implements (or could derive) the
  trait.
- `ignore: Vec<String>` (default `[]`): A list of macro paths the lint should not suggest to
  derive.
