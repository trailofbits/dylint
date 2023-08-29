# assert_eq_arg_misordering

### What it does
Checks for invocations of `assert_eq!` whose arguments are "non-const, const", which
suggests they could be "actual, expected".

### Why is this bad?
In a long list of output, one's eyes naturally go to the last line. Hence, it should be what
is unusual, i.e., the "actual" value.

### Known problems
A common source of false positives is "sorted, unsorted" where the check is of the
sortedness of a collection that is const.

### Example
```rust
assert_eq!(x, 0);
```
Use instead:
```rust
assert_eq!(0, x);
```
