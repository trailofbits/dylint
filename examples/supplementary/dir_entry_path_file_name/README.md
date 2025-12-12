# dir_entry_path_file_name

### What it does

Checks for calling `.path().file_name()` on a `DirEntry` when `.file_name()` can be
called directly.

### Why is this bad?

- For `std::fs::DirEntry`: calling `.path()` allocates a `PathBuf`, which is unnecessary
  when you only need the file name. Additionally, `DirEntry::file_name()` returns an
  `OsString` while `Path::file_name()` returns `Option<&OsStr>` (a more complicated type).
- For `walkdir::DirEntry`: calling `.path().file_name()` returns `Option<&OsStr>` while
  `.file_name()` directly returns `&OsStr` (a simpler type).

### Example

```rust
use std::fs;

for entry in fs::read_dir(".").unwrap() {
    let entry = entry.unwrap();
    let name = entry.path().file_name();
}
```

Use instead:

```rust
use std::fs;

for entry in fs::read_dir(".").unwrap() {
    let entry = entry.unwrap();
    let name = entry.file_name();
}
```
