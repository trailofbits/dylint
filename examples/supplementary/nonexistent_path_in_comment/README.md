# Nonexistent Path in Comment Lint

### What it does

This lint checks comments in the code for file path references and verifies that the referenced files actually do exist. It processes both line comments (using `//`) and block comments (`/* ... */`) by using regex to extract potential file paths. When the lint finds a file path that does not exist on the filesystem it emits a warning.

### Why is this bad?

References to nonexistent files in comments can be misleading:  
- They clutter the code with outdated or inaccurate references.  
- They may cause confusion among developers who are trying to trace implementation details or documentation.  

### Known problems
- Can only check for absolute path or path relative to commented file.
- [This example from the issue](https://github.com/trailofbits/dylint/issues/1225#issue-2315607396) would get flaged even if it did exist, as this lint doesn't check for project root.

### Example

The lint issues a warning when a comment refers to a file that does not exist:

```
// See ../nonexistent/path/file.rs for further details
fn main() {}
```

Use this approach instead, referring to a file that exists:

```
// See ../existing/file.rs for actual implementation details
fn main() {}
```