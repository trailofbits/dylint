# misleading_variable_name

### What it does
Checks for variables satisfying the following three conditions:
- The variable is initialized with the result of a function call.
- The variable's name matches the name of a type defined within the module in which the
  function is defined.
- The variable's type is not the matched type.

### Why is this bad?
A reader could mistakenly believe the variable has a type other than the one it actually
has.

### Example
```rust,no_run
let file = read_to_string(path).unwrap();
```
Use instead:
```rust,no_run
let contents = read_to_string(path).unwrap();
```
