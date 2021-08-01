# Dylint

A tool for running Rust lints from dynamic libraries

```sh
cargo install cargo-dylint
```

Dylint is a Rust linting tool, similar to Clippy. But whereas Clippy runs a predetermined, static set of lints, Dylint runs lints from user-specified, dynamic libraries. Thus, Dylint allows developers to maintain their own personal lint collections.

Note: `cargo-dylint` will not work correctly if installed with the `--debug` flag. If a debug build of `cargo-dylint` is needed, please build it from the `cargo-dylint` package within this repository.

**Contents**

- [Quick start: running Dylint](#quick-start-running-dylint)
- [Quick start: writing lints](#quick-start-writing-lints)
- [How libraries are found](#how-libraries-are-found)
- [Workspace metadata](#workspace-metadata)
- [Library requirements](#library-requirements)
- [Utilities](#utilities)
- [VS Code integration](#vs-code-integration)
- [Limitations](#limitations)
- [Resources](#resources)

## Quick start: running Dylint

The next four commands install Dylint and run one of the example libraries' lints on the Dylint source code:

```sh
cargo install cargo-dylint dylint-link          # Install cargo-dylint and dylint-link
git clone https://github.com/trailofbits/dylint # Clone the Dylint repository
cd dylint                                       # Change directory
cargo dylint allow_clippy                       # Run an example libraries' lint on the Dylint source code
```

In the above example, the library is found via [workspace metadata](#workspace-metadata) (see below).

## Quick start: writing lints

You can start writing your own Dylint libraries by forking the [`dylint-template`](https://github.com/trailofbits/dylint-template) repository. The repository produces a loadable library right out of the box. You can verify this as follows:

```sh
git clone https://github.com/trailofbits/dylint-template
cd dylint-template
cargo build
DYLINT_LIBRARY_PATH=$PWD/target/debug cargo dylint fill_me_in --list
```

All you have to do is implement the [`LateLintPass`](https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html) trait and accommodate the symbols asking to be filled in.

Helpful [resources](#resources) for writing lints appear below.

## How libraries are found

Dylint tries to run all lints in all libraries named on the command line. Dylint resolves names to libraries in the following three ways:

1. Via the `DYLINT_LIBRARY_PATH` environment variable. If `DYLINT_LIBRARY_PATH` is set when Dylint is started, Dylint treats it as a colon-separated list of paths, and searches each path for files with names of the form `DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX` (see [Library requirements](#library-requirements) below). For each such file found, `LIBRARY_NAME` resolves to that file.

2. Via workspace metadata. If Dylint is started in a workspace, Dylint checks the workspace's `Cargo.toml` file for `workspace.metadata.dylint.libraries` (see [Workspace metadata](#workspace-metadata) below). Dylint downloads and builds each listed entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched and names are resolved in the manner described in 1 above.

3. By path. If a name does not resolve to a library via 1 or 2, it is treated as a path.

It is considered an error if a name used on the command line resolves to multiple libraries.

If `--lib name` is used, then `name` is is treated only as a library name, and not as a path.

If `--path name` is used, then `name` is is treated only as a path, and not as a library name.

If `--all` is used, Dylint runs all lints in all libraries discovered via 1 and 2 above.

Note: Earlier versions of Dylint searched the current package's `target/debug` and `target/release` directories for libraries. This feature has been removed.

## Workspace metadata

A workspace can name the libraries it should be linted with in its `Cargo.toml` file. Specifically, a workspace's manifest can contain a TOML list under `workspace.metadata.dylint.libraries`. Each list entry must have the form of a Cargo `git` or `path` dependency, with the following differences:

* There is no leading package name, i.e., no `package =`.
* `path` entries can contain [glob](https://docs.rs/glob/0.3.0/glob/struct.Pattern.html) patterns, e.g., `*`.
* Any entry can contain a `pattern` field whose value is a [glob](https://docs.rs/glob/0.3.0/glob/struct.Pattern.html) pattern. The `pattern` field indicates the subdirectories that contain Dylint libraries.

Dylint downloads and builds each entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched for files with names of the form that Dylint recognizes (see [Library requirements](#library-requirements) below).

As an example, if you include the following in your workspace's `Cargo.toml` file and run `cargo dylint --all --workspace`, Dylint will run all of the example lints in this repository on your workspace:
```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/trailofbits/dylint", pattern = "examples/*" },
]
```

## Library requirements

A Dylint library must satisfy four requirements. **Note:** Before trying to satisfy these explicitly, see [Utilities](#utilities) below.

1. Have a filename of the form:

   ```
   DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX
   ```

   The following is a concrete example on Linux:

   ```
   libquestion_mark_in_expression@nightly-2021-04-08-x86_64-unknown-linux-gnu.so
   ```

   The filename components are as follows:

   - `DLL_PREFIX` and `DLL_SUFFIX` are OS-specific strings. For example, on Linux, they are `lib` and `.so`, respectively.
   - `LIBRARY_NAME` is a name chosen by the library's author.
   - `TOOLCHAIN` is the Rust toolchain for which the library is compiled, e.g., `nightly-2021-04-08-x86_64-unknown-linux-gnu`.

2. Export a `dylint_version` function:

   ```rust
   extern "C" fn dylint_version() -> *mut std::os::raw::c_char
   ```

   This function should return `0.1.0`. This may change in future versions of Dylint.

3. Export a `register_lints` function:

   ```rust
   fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore)
   ```

   This is a function called by the Rust compiler. It is documented [here](https://doc.rust-lang.org/stable/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints).

4. Link against the `rustc_driver` dynamic library. This ensures the library uses Dylint's copies of the Rust compiler crates. This requirement can be satisfied by including the following declaration in your library's `lib.rs` file:
   ```rust
   extern crate rustc_driver;
   ```

Dylint provides [utilities](#utilities) to help meet the above requirements. If your library uses the [`dylint-link`](./dylint-link) tool and the [`dylint_library!`](./utils/linting) macro, then all you should have to do is implement the [`register_lints`](https://doc.rust-lang.org/stable/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints) function.

## Utilities

The following utilities can be helpful for writing Dylint libraries:

- [`dylint-link`](./dylint-link) is a wrapper around Rust's default linker (`cc`) that creates a copy of your library with a filename that Dylint recognizes.
- [`dylint_library!`](./utils/linting) is a macro that automatically defines the `dylint_version` function and adds the `extern crate rustc_driver` declaration.
- [`ui_test`](./utils/testing) is a function that can be used to test Dylint libraries. It provides convenient access to the [`compiletest_rs`](https://github.com/Manishearth/compiletest-rs) package.
- [`clippy_utils`](https://github.com/rust-lang/rust-clippy/tree/master/clippy_utils) is a collection of utilities to make writing lints easier. It is generously made public by the Rust Clippy Developers. Note that, like `rustc`, `clippy_utils` provides no stability guarantees for its APIs.

## VS Code integration

Dylint results can be viewed in VS Code using [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer). To do so, add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.checkOnSave.overrideCommand": [
        "cargo",
        "dylint",
        "--all",
        "--workspace",
        "--",
        "--message-format=json"
    ]
```

If you want to use rust-analyzer inside a lint library, you need add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.rustcSource": "discover",
```

And add this to your `Cargo.toml`:

```toml
[package.metadata.rust-analyzer]
rustc_private = true
```

## Limitations

To run a library's lints on a package, Dylint tries to build the package with the same toolchain used to build the library. So if a package requires a specific toolchain to build, Dylint may not be able to apply certain libraries to that package.

One way this problem can manifest itself is if you try to run one library's lints on the source code of another library. That is, if two libraries use different toolchains, they may not be applicable to each other.

## Resources

Helpful resources for writing lints include the following:

- [Adding a new lint](https://github.com/rust-lang/rust-clippy/blob/master/doc/adding_lints.md) (targeted at Clippy but still useful)
- [Author lint](https://github.com/rust-lang/rust-clippy/blob/master/doc/adding_lints.md#author-lint)
- [Common tools for writing lints](https://github.com/rust-lang/rust-clippy/blob/master/doc/common_tools_writing_lints.md)
- [`rustc_hir` documentation](https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/index.html)
