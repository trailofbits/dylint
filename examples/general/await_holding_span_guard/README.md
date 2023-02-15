# await_holding_span_guard

This lint is due to David Barsky (@davidbarsky).

### What it does
Checks for calls to await while holding a
`tracing` span's `Entered` or `EnteredSpan` guards.

### Why is this bad?
The guards created by `tracing::Span::enter()` or `tracing::Span::entered()` across `.await`
points will result in incorrect traces. This occurs when an async function or async block
yields at an .await point, the current scope is exited, but values in that scope are not
dropped (because the async block will eventually resume execution from that await point).
This means that another task will begin executing while remaining in the entered span.

### Known problems
Will report false positive for explicitly dropped refs ([#6353]).

### Example
```rust,ignore
use tracing::{span, Level};

async fn foo() {
    let span = span!(Level::INFO, "foo");

    THIS WILL RESULT IN INCORRECT TRACES
    let _enter = span.enter();
    bar().await;
}
```

Use instead:
```rust,ignore
use tracing::{span, Level}

async fn foo() {
    let span = span!(Level::INFO, "foo");

    let some_value = span.in_scope(|| {
        // run some synchronous code inside the span...
    });

    // This is okay! The span has already been exited before we reach
    // the await point.
    bar(some_value).await;
}
```

Or use:

```rust,ignore
use tracing::{span, Level, Instrument};

async fn foo() {
    let span = span!(Level::INFO, "foo");
    async move {
        // This is correct! If we yield here, the span will be exited,
        // and re-entered when we resume.
        bar().await;
    }.instrument(span) // instrument the async block with the span...
    .await // ...and await it.
}
```

[#6353]: https://github.com/rust-lang/rust-clippy/issues/6353
