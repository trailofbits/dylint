# format_concat_args

A Dylint lint to suggest using `concat!(...)` instead of `format!(...)` when all format arguments are constant.

This lint identifies instances where `format!` calls could be replaced with more efficient `concat!` calls when all arguments are constant and use the `Display` trait.

## Examples

```rust
// Will trigger the lint:
let s = format!("Hello {}", "world"); // Could be: concat!("Hello ", "world")

// Will not trigger the lint:
let name = "Bob";
let s = format!("Hello {}", name); // Not constant

// Will not trigger the lint:
let s = format!("Value: {:?}", 42); // Not using Display trait
```

## Building and Testing

To build and test this lint:

1. Make sure you have Dylint installed:
   ```sh
   cargo install dylint-link
   ```

2. Build the lint:
   ```sh
   cargo build
   ```

3. To run the lint against your own code:
   ```sh
   DYLINT_LIBRARY_PATH=/path/to/target/debug cargo dylint format_concat_args
   ```

## Limitations

- The current implementation provides a simple detection of `format!` calls but doesn't implement the full type checking and argument analysis.
- Works only with `std::format!` macros, not with other formatting macros like `println!` or `write!`.

## References

- [GitHub Issue #1601](https://github.com/trailofbits/dylint/issues/1601)
