# nonexistent_path_in_comment

### What it does

This lint checks code comments, including both line comments (using `//`) and block comments
(`/*...*/`) for file path references. It then validates that the referenced files exist either
relative to the source file's directory or relative to the workspace root. When a file path
reference does not point to an existing file, the lint emits a warning.

### Why is this bad?

References to nonexistent files in comments can be misleading:

- They clutter the code with outdated or inaccurate references.
- They may cause confusion among developers who are trying to trace implementation details
  or documentation.

### Known problems

Currently, this lint must be allowed at the crate level.

- This example:

```rust
// dylint/dylint/build.rs  (it exists)
```

would get flagged here because the workspace root is `supplementary`
it did exist, as this lint doesn't check for project root.

### Example

```
// See ../nonexistent/path/file.rs for implementation details
fn main() {}
```

Use instead:

```
// See ../actual/path/file.rs for implementation details
fn main() {}
```
