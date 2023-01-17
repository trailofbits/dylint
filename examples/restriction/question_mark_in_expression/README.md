# question_mark_in_expression

### What it does
Checks for `?` operators embedded within a larger expression.

### Why is this bad?
It can be easy to overlook the `?`. Code is more readable when a `?` is the outermost
operator in an expression.

### Example
```rust
Ok(PathBuf::from(&var("PWD")?))
```
Use instead:
```rust
let val = var("PWD")?;
Ok(PathBuf::from(&val))
```
