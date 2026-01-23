# Example Dylint libraries

The example libraries are separated into the following three categories:

- [general] - significant concerns; may produce false positives
- [supplementary] - lesser concerns, but with a low false positive rate
- [restriction] - lesser or stylistic concerns; may produce false positives (similar to [Clippy]'s "restriction" category)
- [experimental] - not ready for primetime yet (similar to [Clippy]'s "nursery" category)
- [testing] - used only for testing purposes

<!-- lint descriptions start -->

## General

| Example                                                                                  | Description/check                                                      |
| ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| [`abs_home_path`](./general/abs_home_path)                                               | String literals that are absolute paths into the user's home directory |
| [`await_holding_span_guard`](./general/await_holding_span_guard)                         | Span guards held while calling await inside an async function          |
| [`basic_dead_store`](./general/basic_dead_store)                                         | A lint to find simple instances of dead stores in arrays               |
| [`crate_wide_allow`](./general/crate_wide_allow)                                         | `#![allow(...)]` used at the crate level                               |
| [`incorrect_matches_operation`](./general/incorrect_matches_operation)                   | Incorrect operators used with matches! macros                          |
| [`non_local_effect_before_error_return`](./general/non_local_effect_before_error_return) | Non-local effects before return of an error                            |
| [`non_thread_safe_call_in_test`](./general/non_thread_safe_call_in_test)                 | Non-thread-safe function calls in tests                                |
| [`wrong_serialize_struct_arg`](./general/wrong_serialize_struct_arg)                     | Calls to serialization methods with incorrect `len` arguments          |

## Supplementary

| Example                                                                                | Description/check                                              |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| [`commented_out_code`](./supplementary/commented_out_code)                             | Code that has been commented out                               |
| [`escaping_doc_link`](./supplementary/escaping_doc_link)                               | Doc comment links that escape their packages                   |
| [`inconsistent_struct_pattern`](./supplementary/inconsistent_struct_pattern)           | Struct patterns whose fields do not match their declared order |
| [`local_ref_cell`](./supplementary/local_ref_cell)                                     | `RefCell` local variables                                      |
| [`nonexistent_path_in_comment`](./supplementary/nonexistent_path_in_comment)           | Lint for nonexistent paths in comments                         |
| [`redundant_reference`](./supplementary/redundant_reference)                           | Reference fields used only to read one copyable subfield       |
| [`unnamed_constant`](./supplementary/unnamed_constant)                                 | Unnamed constants, aka magic numbers                           |
| [`unnecessary_borrow_mut`](./supplementary/unnecessary_borrow_mut)                     | Calls to `RefCell::borrow_mut` that could be `RefCell::borrow` |
| [`unnecessary_conversion_for_trait`](./supplementary/unnecessary_conversion_for_trait) | Unnecessary trait-behavior-preserving calls                    |

## Restriction

| Example                                                                                                      | Description/check                                                                     |
| ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| [`assert_eq_arg_misordering`](./restriction/assert_eq_arg_misordering)                                       | `assert_eq!(actual, expected)`                                                        |
| [`collapsible_unwrap`](./restriction/collapsible_unwrap)                                                     | An `unwrap` that could be combined with an `expect` or `unwrap` using `and_then`      |
| [`const_path_join`](./restriction/const_path_join)                                                           | Joining of constant path components                                                   |
| [`env_literal`](./restriction/env_literal)                                                                   | Environment variables referred to with string literals                                |
| [`inconsistent_qualification`](./restriction/inconsistent_qualification)                                     | Inconsistent qualification of module items                                            |
| [`misleading_variable_name`](./restriction/misleading_variable_name)                                         | Variables whose names suggest they have types other than the ones they have           |
| [`non_topologically_sorted_functions`](./restriction/non_topologically_sorted_functions)                     | A lint to check the function order                                                    |
| [`question_mark_in_expression`](./restriction/question_mark_in_expression)                                   | The `?` operator in expressions                                                       |
| [`ref_aware_redundant_closure_for_method_calls`](./restriction/ref_aware_redundant_closure_for_method_calls) | A ref-aware fork of `redundant_closure_for_method_calls`                              |
| [`register_lints_warn`](./restriction/register_lints_warn)                                                   | Calls to `rustc_errors::DiagCtxtHandle::warn` from within a `register_lints` function |
| [`suboptimal_pattern`](./restriction/suboptimal_pattern)                                                     | Patterns that could perform additional destructuring                                  |
| [`try_io_result`](./restriction/try_io_result)                                                               | The `?` operator applied to `std::io::Result`                                         |

## Experimental

| Example                                                             | Description/check                              |
| ------------------------------------------------------------------- | ---------------------------------------------- |
| [`derive_opportunity`](./experimental/derive_opportunity)           | Traits that could be derived                   |
| [`missing_doc_comment_llm`](./experimental/missing_doc_comment_llm) | A lint that suggests doc comments using an LLM |

## Testing

| Example                            | Description/check                                      |
| ---------------------------------- | ------------------------------------------------------ |
| [`clippy`](./testing/clippy)       | All of the Clippy lints as a Dylint library            |
| [`straggler`](./testing/straggler) | A lint that uses an old toolchain for testing purposes |

<!-- lint descriptions end -->

**Notes**

1. Each example is in its own workspace so that it can have its own `rust-toolchain`.
2. Each example is configured to use the installed copy of [`dylint-link`](../dylint-link). To use the copy within this repository, change the example's `.cargo/config.toml` file as follows:
   ```toml
   [target.x86_64-unknown-linux-gnu]
   linker = "../../../target/debug/dylint-link"
   ```

[clippy]: https://github.com/rust-lang/rust-clippy#clippy
[experimental]: #experimental
[general]: #general
[restriction]: #restriction
[supplementary]: #supplementary
[testing]: #testing
