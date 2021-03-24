# question_mark_in_expression

**What it does:** Checks for `?` operators embedded within a larger expression.

**Why is this bad?** It can be easy to overlook the `?`. Code is more readable when a `?` is
the outermost operator in an expression.

**Known problems:** None.

**Example:**

```rust
Ok(std::path::PathBuf::from(&std::env::var("PWD")?))
```
Use instead:
```rust
let val = std::env::var("PWD")?;
Ok(std::path::PathBuf::from(&val))
```
