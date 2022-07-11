# Example Dylint libraries

The example libraries are separated into the following three categories:

- [general] - applicable to most projects
- [restriction] - would likely be considered "restriction lints" by [Clippy], e.g., reflect concerns not necessarily held by all authors
- [testing] - used only for testing purposes

## General

| Example                                                                                  | Description                                                                        |
| ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`await_holding_span_guard`](./general/await_holding_span_guard)                         | A lint to check for Span guards held while calling await inside an async function  |
| [`crate_wide_allow`](./general/crate_wide_allow)                                         | A lint to check for `#![allow(...)]` used at the crate level                       |
| [`env_cargo_path`](./general/env_cargo_path)                                             | A lint to check for `env!` applied to Cargo environment variables containing paths |
| [`non_local_effect_before_error_return`](./general/non_local_effect_before_error_return) | A lint to check for non-local effects before return of an error                    |
| [`non_thread_safe_call_in_test`](./general/non_thread_safe_call_in_test)                 | A lint to check for non-thread-safe function calls in tests                        |
| [`redundant_reference`](./general/redundant_reference)                                   | A lint to check for reference fields used only to read one copyable subfield       |

## Restriction

| Example                                                                              | Description                                                                |
| ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| [`env_literal`](./restriction/env_literal)                                           | A lint to check for environment variables referred to with string literals |
| [`inconsistent_qualification`](./restriction/inconsistent_qualification)             | A lint to check for inconsistent qualification of module items             |
| [`path_separator_in_string_literal`](./restriction/path_separator_in_string_literal) | A lint to check for path separators in string literals                     |
| [`question_mark_in_expression`](./restriction/question_mark_in_expression)           | A lint to check for the `?` operator in expressions                        |
| [`suboptimal_pattern`](./restriction/suboptimal_pattern)                             | A lint to check for patterns that could perform additional destructuring   |
| [`try_io_result`](./restriction/try_io_result)                                       | A lint to check for the `?` operator applied to `std::io::Result`          |

## Testing

| Example                            | Description                                            |
| ---------------------------------- | ------------------------------------------------------ |
| [`clippy`](./testing/clippy)       | All of the Clippy lints as a Dylint library            |
| [`straggler`](./testing/straggler) | A lint that uses an old toolchain for testing purposes |

**Notes**

1. Each example is in its own workspace so that it can have its own `rust-toolchain`.
2. Each example is configured to use the installed copy of [`dylint-link`](../dylint-link). To use the copy within this repository, change the example's `.cargo/config.toml` file as follows:
   ```toml
   [target.x86_64-unknown-linux-gnu]
   linker = "../../../target/debug/dylint-link"
   ```

[clippy]: https://github.com/rust-lang/rust-clippy#clippy
[general]: #general
[restriction]: #restriction
[testing]: #testing
