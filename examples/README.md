# Example Dylint libraries

The example libraries are separated into the following three categories:

- [general] - significant concerns; may produce false positives
- [supplementary] - lesser concerns, but with a low false positive rate
- [restriction] - lesser or stylistic concerns; may produce false positives (similar to [Clippy]'s "restriction" category)
- [testing] - used only for testing purposes

## General

| Example                                                                                  | Description/check                                              |
| ---------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| [`await_holding_span_guard`](./general/await_holding_span_guard)                         | Span guards held while calling await inside an async function  |
| [`crate_wide_allow`](./general/crate_wide_allow)                                         | `#![allow(...)]` used at the crate level                       |
| [`env_cargo_path`](./general/env_cargo_path)                                             | `env!` applied to Cargo environment variables containing paths |
| [`non_local_effect_before_error_return`](./general/non_local_effect_before_error_return) | Non-local effects before return of an error                    |
| [`non_thread_safe_call_in_test`](./general/non_thread_safe_call_in_test)                 | Non-thread-safe function calls in tests                        |

## Supplementary

| Example                                                                                | Description/check                                              |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| [`commented_code`](./supplementary/commented_code)                                     | Code that has been commented out                               |
| [`redundant_reference`](./supplementary/redundant_reference)                           | Reference fields used only to read one copyable subfield       |
| [`unnamed_constant`](./supplementary/unnamed_constant)                                 | Unnamed constants, aka magic numbers                           |
| [`unnecessary_borrow_mut`](./supplementary/unnecessary_borrow_mut)                     | Calls to `RefCell::borrow_mut` that could be `RefCell::borrow` |
| [`unnecessary_conversion_for_trait`](./supplementary/unnecessary_conversion_for_trait) | Unnecessary trait-behavior-preserving calls                    |

## Restriction

| Example                                                                                                      | Description/check                                                                |
| ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| [`collapsible_unwrap`](./restriction/collapsible_unwrap)                                                     | An `unwrap` that could be combined with an `expect` or `unwrap` using `and_then` |
| [`const_path_join`](./restriction/const_path_join)                                                           | Joining of constant path components                                              |
| [`derive_opportunity`](./restriction/derive_opportunity)                                                     | Traits that could be derived                                                     |
| [`env_literal`](./restriction/env_literal)                                                                   | Environment variables referred to with string literals                           |
| [`inconsistent_qualification`](./restriction/inconsistent_qualification)                                     | Inconsistent qualification of module items                                       |
| [`misleading_variable_name`](./restriction/misleading_variable_name)                                         | Variables whose names suggest they have types other than the ones they have      |
| [`missing_doc_comment_openai`](./restriction/missing_doc_comment_openai)                                     | A lint that suggests doc comments using OpenAI                                   |
| [`overscoped_allow`](./restriction/overscoped_allow)                                                         | `allow` attributes whose scope could be reduced                                  |
| [`question_mark_in_expression`](./restriction/question_mark_in_expression)                                   | The `?` operator in expressions                                                  |
| [`ref_aware_redundant_closure_for_method_calls`](./restriction/ref_aware_redundant_closure_for_method_calls) | A ref-aware fork of `redundant_closure_for_method_calls`                         |
| [`suboptimal_pattern`](./restriction/suboptimal_pattern)                                                     | Patterns that could perform additional destructuring                             |
| [`try_io_result`](./restriction/try_io_result)                                                               | The `?` operator applied to `std::io::Result`                                    |

## Testing

| Example                            | Description/check                                      |
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
[supplementary]: #supplementary
[testing]: #testing
