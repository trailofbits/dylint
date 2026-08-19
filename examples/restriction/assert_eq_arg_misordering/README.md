# assert_eq_arg_misordering

### What it does

Checks for invocations of `assert_eq!` or `assert_ne!` whose arguments are "non-const,
const-like", which suggests they could be "actual, expected".

An argument is "const-like" if it is const-evaluatable, or if it is a non-empty `vec!`
invocation whose arguments are const-like. The latter allowance is needed because a `vec!`
invocation allocates, and is therefore never const-evaluatable, even when its value is
determined entirely by the source text. An empty `vec!` is excluded because moving one to
the first position can leave its element type uninferable, e.g., `assert_eq!(x, vec![])`
compiles but `assert_eq!(vec![], x)` does not.

### Why is this bad?

In a long list of output, one's eyes naturally go to the last line. Hence, it should be
what is unusual, i.e., the "actual" value.

### Known problems

A common source of false positives is "sorted, unsorted" where the check is of the
sortedness of a collection that is const-like.

### Example

```rust
assert_eq!(x, 0);
```

Use instead:

```rust
assert_eq!(0, x);
```
