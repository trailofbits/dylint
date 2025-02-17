# register_lints_warn

### What it does

Checks for calls to `rustc_errors::DiagCtxtHandle::warn` from within a `register_lints`
function.

### Why is this bad?

Dylint lists a library's lints by calling the library's `register_lints` function and
comparing the lints that are registered before and after the call. If the library's
`register_lints` functions emits warnings, they will be emitted when a user tries to list
the library's lints.

### Example

```rust
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    if condition {
        sess.dcx().warn("something bad happened");
    }
}
```

Use instead:

```rust
impl<'tcx> rustc_lint::LateLintPass<'tcx> for LintPass {
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        if condition {
            cx.sess().dcx().warn("something bad happened");
        }
    }
}
```
