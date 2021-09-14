# dylint_testing

This crate provides convenient access to the [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) package for testing [Dylint](https://github.com/trailofbits/dylint) libraries.

Specifically, this crate provides the following three functions. Note: If your test has dependencies, you must use `ui_test_example` or `ui_test_examples`. See the [question_mark_in_expression](../../examples/question_mark_in_expression/Cargo.toml) example in this repository.

- `ui_test` - test a library on all source files in a directory

  ```rust
  pub fn ui_test(name: &str, src_base: &Path)
  ```

  - `name` is the name of a Dylint library to be tested. (Often, this is the same as the package name.)
  - `src_base` is a directory containing:
    - source files on which to test the library (`.rs` files), and
    - the output those files should produce (`.stderr` files).

- `ui_test_example` - test a library on one example target

  ```rust
  pub fn ui_test_example(name: &str, example: &str)
  ```

  - `name` is the name of a Dylint library to be tested.
  - `example` is an example target on which to test the library.

- `ui_test_examples` - test a library on all example targets
  ```rust
  pub fn ui_test_examples(name: &str)
  ```
  - `name` is the name of a Dylint library to be tested.

For most situations, you can add the following to your library's `lib.rs` file:

```rust
#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
```

And include one or more `.rs` and `.stderr` files in a `ui` directory alongside your library's `src` directory. See the [examples](../../examples) in this repository.

## Updating `.stderr` files

If the standard error that results from running your `.rs` file differs from the contents of your `.stderr` file, `compiletest_rs` will produce a report like the following:

```rust
diff of stderr:

 error: calling `std::env::set_var` in a test could affect the outcome of other tests
   --> $DIR/main.rs:8:5
    |
 LL |     std::env::set_var("KEY", "VALUE");
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `-D nonreentrant-function-in-test` implied by `-D warnings`

-error: aborting due to previous error
+error: calling `std::env::set_var` in a test could affect the outcome of other tests
+  --> $DIR/main.rs:23:9
+   |
+LL |         std::env::set_var("KEY", "VALUE");
+   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
+
+error: aborting due to 2 previous errors



The actual stderr differed from the expected stderr.
Actual stderr saved to ...
```

The meaning of each line is as follows:

- A line beginning with a plus (`+`) is in the actual standard error, but not in your `.stderr` file.
- A line beginning with a minus (`-`) is in your `.stderr` file, but not in the actual standard error.
- A line beginning with a space (` `) is in both the actual standard error and your `.stderr` file, and is provided for context.
- All other lines (e.g., `diff of stderr:`) contain `compiletest_rs` messages.

**Note:** In the actual standard error, a blank line usually follows the `error: aborting due to N previous errors` line. So a correct `.stderr` file will typically contain one blank line at the end.

In general, it is not too hard to update a `.stderr` file by hand. However, the `compiletest_rs` report should contain a line of the form `Actual stderr saved to PATH`. Copying `PATH` to your `.stderr` file should update it completely.

Additional documentation on `compiletest_rs` can be found in [its repository](https://github.com/Manishearth/compiletest-rs).
