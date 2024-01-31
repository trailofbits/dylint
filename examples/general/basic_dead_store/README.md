# basic_dead_store

### What it does
Finds instances of dead stores in arrays: array positions that are assigned twice without a
use or read in between.

### Why is this bad?
A dead store might indicate a logic error in the program or an unnecessary assignment.

### Known problems
This lint only checks for literal indices and will not try to find instances where an array
is indexed by a variable.

### Example
```rust
let mut arr = [0u64; 2];
arr[0] = 1;
arr[0] = 2;
```
Use instead:
```rust
let mut arr = [0u64; 2];
arr[0] = 2;
arr[1] = 1;
```
