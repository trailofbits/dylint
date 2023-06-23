# try_io_result

### What it does
Checks for `?` operators applied to values of type `std::io::Result`.

### Why is this bad?
Returning a `std::io::Result` could mean relevant context (e.g., files or paths involved) is
lost. The problem is discussed under "Verbose IO errors" in Yoshua Wuyts' [Error Handling
Survey].

### Known problems
No interprocedural analysis is done. So if context is added by the caller, it will go
unnoticed.

### Example
```rust
fn foo() -> std::io::Result<()> {
    let _ = File::open("/dev/null")?;
    Ok(())
}
```
Use instead:
```rust
use anyhow::Context;
fn foo() -> anyhow::Result<()> {
    let _ = File::open("/dev/null").with_context(|| "could not open `/dev/null`")?;
    Ok(())
}
```

[error handling survey]: https://blog.yoshuawuyts.com/error-handling-survey/
