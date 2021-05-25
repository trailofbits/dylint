# dylint_testing

This crate provides convenient access to the [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) package for testing [Dylint](https://github.com/trailofbits/dylint) libraries.

Specifically, this crate provides the following three functions. Note: If your test has dependencies, you must use `ui_test_example` or `ui_test_examples`. See the [question_mark_in_expression](../../examples/question_mark_in_expression/Cargo.toml) example in this repository.

* `ui_test` - test a library on all source files in a directory
    ```rust
    pub fn ui_test(name: &str, src_base: &Path)
    ```
    * `name` is the name of a Dylint library to be tested. (Often, this is the same as the package name.)
    * `src_base` is a directory containing:
        * source files on which to test the library (`.rs` files), and
        * the output those files should produce (`.stderr` files).

* `ui_test_example` - test a library on one example target
    ```rust
    pub fn ui_test_example(name: &str, example: &str)
    ```
    * `name` is the name of a Dylint library to be tested.
    * `example` is an example target on which to test the library.

* `ui_test_examples` - test a library on all example targets
    ```rust
    pub fn ui_test_example(name: &str)
    ```
    * `name` is the name of a Dylint library to be tested.

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

Additional documentation on [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) can be found in its repository.
