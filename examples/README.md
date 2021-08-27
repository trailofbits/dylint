# Example Dylint libraries

Each subdirectory contains an example [Dylint](https://github.com/trailofbits/dylint) library.

The current examples are:

| Example                                                                  | Description                                                                |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| [`allow_clippy`](./allow_clippy)                                         | A tongue-in-cheek example of a Dylint library                              |
| [`clippy`](./clippy)                                                     | All of the Clippy lints as a Dylint library                                |
| [`env_literal`](./env_literal)                                           | A lint to check for environment variables referred to with string literals |
| [`nonreentrant_function_in_test`](./nonreentrant_function_in_test)       | A lint to check for nonreentrant functions in tests                        |
| [`path_separator_in_string_literal`](./path_separator_in_string_literal) | A lint to check for path separators in string literals                     |
| [`question_mark_in_expression`](./question_mark_in_expression)           | A lint to check for the `?` operator in expressions                        |
| [`try_io_result`](./try_io_result)                                       | A lint to check for the `?` operator applied to `std::io::Result`          |

**Notes**

1. Each example is in its own workspace so that it can have its own `rust-toolchain`.
2. Each example is configured to use the installed copy of [`dylint-link`](../dylint-link). To use the copy within this repository, change the example's `.cargo/config.toml` file as follows:
   ```toml
   [target.x86_64-unknown-linux-gnu]
   linker = "../../target/debug/dylint-link"
   ```
