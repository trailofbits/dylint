# dylint_testing

[docs.rs documentation]

<!-- cargo-rdme start -->

This crate provides convenient access to the [`compiletest_rs`] package for testing [Dylint]
libraries.

**Note: If your test has dependencies, you must use `ui_test_example` or `ui_test_examples`.**
See the [`question_mark_in_expression`] example in this repository.

This crate provides the following three functions:

- [`ui_test`] - test a library on all source files in a directory
- [`ui_test_example`] - test a library on one example target
- [`ui_test_examples`] - test a library on all example targets

For most situations, you can add the following to your library's `lib.rs` file:

```rust
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
```

And include one or more `.rs` and `.stderr` files in a `ui` directory alongside your library's
`src` directory. See the [examples] in this repository.

## Test builder

In addition to the above three functions, [`ui::Test`] is a test "builder." Currently, the main
advantage of using `Test` over the above functions is that `Test` allows flags to be passed to
`rustc`. For an example of its use, see [`non_thread_safe_call_in_test`] in this repository.

`Test` has three constructors, which correspond to the above three functions as follows:

- [`ui::Test::src_base`] <-> [`ui_test`]
- [`ui::Test::example`] <-> [`ui_test_example`]
- [`ui::Test::examples`] <-> [`ui_test_examples`]

In each case, the constructor's arguments are exactly those of the corresponding function.

A `Test` instance has the following methods:

- `dylint_toml` - set the `dylint.toml` file's contents (for testing [configurable libraries])
- `rustc_flags` - pass flags to the compiler when running the test
- `run` - run the test

## Updating `.stderr` files

If the standard error that results from running your `.rs` file differs from the contents of
your `.stderr` file, `compiletest_rs` will produce a report like the following:

```text
diff of stderr:

 error: calling `std::env::set_var` in a test could affect the outcome of other tests
   --> $DIR/main.rs:8:5
    |
 LL |     std::env::set_var("KEY", "VALUE");
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: `-D non-thread-safe-call-in-test` implied by `-D warnings`

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

- A line beginning with a plus (`+`) is in the actual standard error, but not in your `.stderr`
  file.
- A line beginning with a minus (`-`) is in your `.stderr` file, but not in the actual standard
  error.
- A line beginning with a space (` `) is in both the actual standard error and your `.stderr`
  file, and is provided for context.
- All other lines (e.g., `diff of stderr:`) contain `compiletest_rs` messages.

**Note:** In the actual standard error, a blank line usually follows the `error: aborting due to
N previous errors` line. So a correct `.stderr` file will typically contain one blank line at
the end.

In general, it is not too hard to update a `.stderr` file by hand. However, the `compiletest_rs`
report should contain a line of the form `Actual stderr saved to PATH`. Copying `PATH` to your
`.stderr` file should update it completely.

Additional documentation on `compiletest_rs` can be found in [its repository].

[Dylint]: https://github.com/trailofbits/dylint/tree/master
[`compiletest_rs`]: https://github.com/Manishearth/compiletest-rs
[`non_thread_safe_call_in_test`]: https://github.com/trailofbits/dylint/tree/master/examples/general/non_thread_safe_call_in_test/src/lib.rs
[`question_mark_in_expression`]: https://github.com/trailofbits/dylint/tree/master/examples/restriction/question_mark_in_expression/Cargo.toml
[`ui::Test::example`]: https://docs.rs/dylint_testing/latest/dylint_testing/ui/struct.Test.html#method.example
[`ui::Test::examples`]: https://docs.rs/dylint_testing/latest/dylint_testing/ui/struct.Test.html#method.examples
[`ui::Test::src_base`]: https://docs.rs/dylint_testing/latest/dylint_testing/ui/struct.Test.html#method.src_base
[`ui::Test`]: https://docs.rs/dylint_testing/latest/dylint_testing/ui/struct.Test.html
[`ui_test_example`]: https://docs.rs/dylint_testing/latest/dylint_testing/fn.ui_test_example.html
[`ui_test_examples`]: https://docs.rs/dylint_testing/latest/dylint_testing/fn.ui_test_examples.html
[`ui_test`]: https://docs.rs/dylint_testing/latest/dylint_testing/fn.ui_test.html
[configurable libraries]: https://github.com/trailofbits/dylint/tree/master#configurable-libraries
[docs.rs documentation]: https://docs.rs/dylint_testing/latest/dylint_testing/
[examples]: https://github.com/trailofbits/dylint/tree/master/examples
[its repository]: https://github.com/Manishearth/compiletest-rs

<!-- cargo-rdme end -->
