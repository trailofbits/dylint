# dylint_testing

This crate provides a `ui_test` function for testing [Dylint](https://github.com/trailofbits/dylint) libraries.

`ui_test` provides convenient access to the [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) package. `ui_test` is declared as follows:

```rust
pub fn ui_test(name: &str, src_base: &Path)
```

Its arguments are as follows:

* `name` is the name of a Dylint library to be tested. Often, this is the same as the package name.
* `src_base` is a directory containing:
    * source files on which to test the library (`.rs` files), and
    * the output those files should produce (`.stderr` files).

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

And include one or more `.rs` and `.stderr` files in a `ui` directory alongside your library's `src` directory. See the [examples](../examples) in this repository.

Additional documentation on [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) can be found in its repository.
