# Test case for false positive in `unnecessary_conversion_for_trait` lint

This PR adds a test case demonstrating the bug reported in issue #1531, where the `unnecessary_conversion_for_trait` lint incorrectly suggests removing `.iter()` calls in cases where the original collection is needed later.

## Test Case

Added a test file `false_positive.rs` with the `#[allow(unnecessary_conversion_for_trait)]` attribute that reproduces the issue:

```rust
#[allow(unnecessary_conversion_for_trait)]
fn main() {
    let xs = vec![[0u8; 16]];
    let mut ys: Vec<[u8; 16]> = Vec::new();
    ys.extend(xs.iter());
    //          ^^^^^^^
    println!("{:?}", xs);
}
```

Without the `#[allow]` attribute, the lint would incorrectly suggest removing `.iter()`, which would consume `xs` and make it unavailable for the `println!` statement.

## Implementation

Added a test function in `lib.rs` to run this test:

```rust
#[test]
fn false_positive() {
    let _lock = MUTEX.lock().unwrap();
    assert!(!enabled("COVERAGE"));
    assert!(!enabled("CHECK_INHERENTS"));
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "false_positive");
}
```

This test case validates the existence of the issue, as requested in #1531. 