# path_separator_in_string_literal

**What it does:** Checks for path separators (e.g., `/`) in string literals.

**Why is this bad?** Path separators can vary from one OS to another. Including them in
a string literal is not portable.

**Known problems:** None.

**Example:**

```rust
PathBuf::from("../target")
```
Use instead:
```rust
PathBuf::from("..").join("target")
```
