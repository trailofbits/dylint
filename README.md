# Dylint

A tool for running Rust lints from dynamic libraries

```sh
cargo install cargo-dylint --version '>=0.1.0-pre'
```

Dylint is a Rust linting tool, similar to Clippy. But whereas Clippy runs a predetermined, static set of lints, Dylint runs lints from user-specified, dynamic libraries. Thus, Dylint allows developers to have their own personal lint collections.

**Contents**

* [Quick start](#quick-start)
* [Library requirements](#library-requirements)
* [How libraries are found](#how-libraries-are-found)
* [Utilities](#utilities)
* [References](#references)

## Quick start

```sh
cargo install cargo-dylint dylint-link --version '>=0.1.0-pre' # Install cargo-dylint and dylint-link
git clone https://github.com/trailofbits/dylint                # Clone the Dylint repository
cd dylint/examples/allow_clippy                                # Go to one of the example lint libraries
cargo build                                                    # Build the library
cargo dylint allow_clippy -- --manifest-path ../../Cargo.toml  # Run the library's lint on the Dylint source code
```

You can start writing your own Dylint libraries by forking the [`dylint-template`](https://github.com/trailofbits/dylint-template) repository.

## Library requirements

A Dylint library must satisfy four requirements. **Note:** before trying to satisfy these explicitly, see [Utilities](#utilities) below.

1. Have a filename of the form:
    ```
    DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX
    ```
    The following is a concrete example on Linux:
    ```
    libquestion_mark_in_expression@nightly-2021-03-11-x86_64-unknown-linux-gnu.so
    ```
    The filename components are as follows:
    * `DLL_PREFIX` and `DLL_SUFFIX` are OS-specific strings. For example, on Linux, they are `lib` and `.so`, respectively.
    * `LIBARY_NAME` is a name chosen by the library's author.
    * `TOOLCHAIN` is the Rust toolchain for which the library is compiled, e.g., `nightly-2021-03-11-x86_64-unknown-linux-gnu`.

2. Export a `dylint_version` function:
    ```rust
    extern "C" fn dylint_version() -> *mut std::os::raw::c_char
    ```
    This function should return the Dylint version the library is compiled for. The current Dylint version is `0.1.0-pre.2`.

3. Export a `register_lints` function:
    ```rust
    fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore)
    ```
    This is a function called by the Rust compiler. It is documented [here](https://doc.rust-lang.org/stable/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints).

4. Link against the `rustc_driver` dynamic library. This ensures the library uses Dylint's copies of the Rust compiler crates. This requirement can be satisfied by including the following declaration in your libraries `lib.rs` file:
    ```rust
    extern crate rustc_driver;
    ```

Dylint provides [utilities](#utilities) to help meet the above requirements. If your library uses the [`dylint-link`](./dylint-link) tool and the [`dylint_library!`](./utils/linting) macro, then all you should have to do is implement the `register_lints` function.

## How libraries are found

When Dylint is started, the following locations are searched:

* the colon-separated paths in `DYLINT_LIBRARY_PATH` (if set)
* the current package's `target/debug` directory (if in a package)
* the current package's `target/release` directory (if in a package)

Any file found in the above locations with a name of the form `DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX` is considered a Dylint library.

In an invocation of the form `cargo dylint [names]`, each `name` in `names` is compared to the libraries found in the above manner. If `name` matches a discovered library's `LIBRARY_NAME`, then `name` resolves to that library. It is considered an error if a `name` resolves to multiple libraries.

If the above process does not resolve `name` to a library, then `name` is treated as a path.

If `--lib name` is used, then `name` is is treated only as a library name, and not as a path.

If `--path name` is used, then `name` is is treated only as a path, and not as a library name.

## Utilities

The following utilities can be helpful for writing Dylint libraries:

* [`dylint-link`](./dylint-link) is a wrapper around Rust's default linker (`cc`) that creates a copy of your library with a filename that Dylint recognizes.
* [`dylint_library!`](./utils/linting) is a macro that automatically defines the `dylint_version` function and adds the `extern crate rustc_driver` declaration.
* [`ui_test`](./utils/testing) is a function that can be used to test Dylint libraries. It provides convenient access to the [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) package.
* [`clippy_utils`](https://github.com/rust-lang/rust-clippy/tree/master/clippy_utils) is a collection of utilities to make writing lints easier. It is generously provided by the Rust Clippy Developers.

## References

Useful references for writing lints include:

* [Adding a new lint](https://github.com/rust-lang/rust-clippy/blob/master/doc/adding_lints.md) (targeted at Clippy, but still useful)
* [Common tools for writing lints](https://github.com/rust-lang/rust-clippy/blob/master/doc/common_tools_writing_lints.md)
* [`rustc_hir` documentation](https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/index.html)
