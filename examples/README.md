# Example Dylint libraries

Each subdirectory contains an example [Dylint](https://github.com/trailofbits/dylint) library.

The current examples are:

| Example                                                                  | Description                                                                        |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| [`await_holding_span_guard`](./await_holding_span_guard)                 | A lint to check for Span guards held while calling await inside an async function  |
| [`clippy`](./clippy)                                                     | All of the Clippy lints as a Dylint library                                        |
| [`crate_wide_allow`](./crate_wide_allow)                                 | A lint to check for `#![allow(...)]` used at the crate level                       |
| [`env_cargo_path`](./env_cargo_path)                                     | A lint to check for `env!` applied to Cargo environment variables containing paths |
| [`env_literal`](./env_literal)                                           | A lint to check for environment variables referred to with string literals         |
| [`non_thread_safe_call_in_test`](./non_thread_safe_call_in_test)         | A lint to check for non-thread-safe function calls in tests                        |
| [`path_separator_in_string_literal`](./path_separator_in_string_literal) | A lint to check for path separators in string literals                             |
| [`question_mark_in_expression`](./question_mark_in_expression)           | A lint to check for the `?` operator in expressions                                |
| [`try_io_result`](./try_io_result)                                       | A lint to check for the `?` operator applied to `std::io::Result`                  |

**Notes**

1. Each example is in its own workspace so that it can have its own `rust-toolchain`.
2. Each example is configured to use the installed copy of [`dylint-link`](../dylint-link). To use the copy within this repository, change the example's `.cargo/config.toml` file as follows:
   ```toml
   [target.x86_64-unknown-linux-gnu]
   linker = "../../target/debug/dylint-link"
   ```
