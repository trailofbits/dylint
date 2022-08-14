# Dylint

A tool for running Rust lints from dynamic libraries

```sh
cargo install cargo-dylint dylint-link
```

Dylint is a Rust linting tool, similar to Clippy. But whereas Clippy runs a predetermined, static set of lints, Dylint runs lints from user-specified, dynamic libraries. Thus, Dylint allows developers to maintain their own personal lint collections.

**Contents**

- [Quick start]
  - [Running Dylint]
  - [Writing lints]
- [How libraries are found]
- [Workspace metadata]
- [Conditional compilation]
- [Library requirements]
- [Utilities]
- [VS Code integration]
- [Limitations]
- [Resources]

## Quick start

### Running Dylint

The next three steps install Dylint and run all of this repository's [example lints] on a workspace:

1. Install `cargo-dylint` and `dylint-link`:

   ```sh
   cargo install cargo-dylint dylint-link
   ```

2. Add the following to the workspace's `Cargo.toml` file:

   ```toml
   [workspace.metadata.dylint]
   libraries = [
       { git = "https://github.com/trailofbits/dylint", pattern = "examples/*/*" },
   ]
   ```

3. Run `cargo-dylint`:
   ```sh
   cargo dylint --all --workspace
   ```

In the above example, the libraries are found via [workspace metadata] (see below).

### Writing lints

You can start writing your own Dylint library by running `cargo dylint new new_lint_name`. Doing so will produce a loadable library right out of the box. You can verify this as follows:

```sh
cargo dylint new new_lint_name
cd new_lint_name
cargo build
DYLINT_LIBRARY_PATH=$PWD/target/debug cargo dylint list --lib new_lint_name
```

All you have to do is implement the [`LateLintPass`] trait and accommodate the symbols asking to be filled in.

Helpful [resources] for writing lints appear below.

## How libraries are found

Dylint tries to run all lints in all libraries named on the command line. Dylint resolves names to libraries in the following three ways:

1. Via the `DYLINT_LIBRARY_PATH` environment variable. If `DYLINT_LIBRARY_PATH` is set when Dylint is started, Dylint treats it as a colon-separated list of paths, and searches each path for files with names of the form `DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX` (see [Library requirements] below). For each such file found, `LIBRARY_NAME` resolves to that file.

2. Via workspace metadata. If Dylint is started in a workspace, Dylint checks the workspace's `Cargo.toml` file for `workspace.metadata.dylint.libraries` (see [Workspace metadata] below). Dylint downloads and builds each listed entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched and names are resolved in the manner described in 1 above.

3. By path. If a name does not resolve to a library via 1 or 2, it is treated as a path.

It is considered an error if a name used on the command line resolves to multiple libraries.

If `--lib name` is used, then `name` is is treated only as a library name, and not as a path.

If `--path name` is used, then `name` is is treated only as a path, and not as a library name.

If `--all` is used, Dylint runs all lints in all libraries discovered via 1 and 2 above.

Note: Earlier versions of Dylint searched the current package's `target/debug` and `target/release` directories for libraries. This feature has been removed.

## Workspace metadata

A workspace can name the libraries it should be linted with in its `Cargo.toml` file. Specifically, a workspace's manifest can contain a TOML list under `workspace.metadata.dylint.libraries`. Each list entry must have the form of a Cargo `git` or `path` dependency, with the following differences:

- There is no leading package name, i.e., no `package =`.
- `path` entries can contain [glob] patterns, e.g., `*`.
- Any entry can contain a `pattern` field whose value is a [glob] pattern. The `pattern` field indicates the subdirectories that contain Dylint libraries.

Dylint downloads and builds each entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched for files with names of the form that Dylint recognizes (see [Library requirements] below).

As an example, if you include the following in your workspace's `Cargo.toml` file and run `cargo dylint --all --workspace`, Dylint will run on your workspace all of this repository's [example general lints], as well as the example restriction lint [`try_io_result`].

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/trailofbits/dylint", pattern = "examples/general/*" },
    { git = "https://github.com/trailofbits/dylint", pattern = "examples/restriction/try_io_result" },
]
```

## Conditional compilation

For each library that Dylint uses to check a crate, Dylint passes the following to the Rust compiler:

```sh
--cfg=dylint_lib="LIBRARY_NAME"
```

You can use this feature to allow a lint when Dylint is used, but also avoid an "unknown lint" warning when Dylint is not used. Specifically, you can do the following:

```rust
#[cfg_attr(dylint_lib = "LIBRARY_NAME", allow(LINT_NAME))]
```

Note that `LIBRARY_NAME` and `LINT_NAME` may be the same. For an example involving [`non_thread_safe_call_in_test`], see [dylint/src/lib.rs] in this repository.

Also note that the just described approach does not work for pre-expansion lints. The only known workaround for pre-expansion lints is allow the compiler's built-in [`unknown_lints`] lint. Specifically, you can do the following:

```rust
#[allow(unknown_lints)]
#[allow(PRE_EXPANSION_LINT_NAME)]
```

For an example involving [`env_cargo_path`], see [internal/src/examples.rs] in this repository.

## Library requirements

A Dylint library must satisfy four requirements. **Note:** Before trying to satisfy these explicitly, see [Utilities] below.

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

   This is a function called by the Rust compiler. It is documented [here].

4. Link against the `rustc_driver` dynamic library. This ensures the library uses Dylint's copies of the Rust compiler crates. This requirement can be satisfied by including the following declaration in your library's `lib.rs` file:
   ```rust
   extern crate rustc_driver;
   ```

Dylint provides [utilities] to help meet the above requirements. If your library uses the [`dylint-link`] tool and the [`dylint_library!`] macro, then all you should have to do is implement the [`register_lints`] function.

## Utilities

The following utilities can be helpful for writing Dylint libraries:

- [`dylint-link`] is a wrapper around Rust's default linker (`cc`) that creates a copy of your library with a filename that Dylint recognizes.
- [`dylint_library!`] is a macro that automatically defines the `dylint_version` function and adds the `extern crate rustc_driver` declaration.
- [`ui_test`] is a function that can be used to test Dylint libraries. It provides convenient access to the [`compiletest_rs`] package.
- [`clippy_utils`] is a collection of utilities to make writing lints easier. It is generously made public by the Rust Clippy Developers. Note that, like `rustc`, `clippy_utils` provides no stability guarantees for its APIs.

## VS Code integration

Dylint results can be viewed in VS Code using [rust-analyzer]. To do so, add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.checkOnSave.overrideCommand": [
        "cargo",
        "dylint",
        "--all",
        "--workspace",
        "--",
        "--all-targets",
        "--message-format=json"
    ]
```

If you want to use rust-analyzer inside a lint library, you need to add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.rustcSource": "discover",
```

And add the following to the library's `Cargo.toml` file:

```toml
[package.metadata.rust-analyzer]
rustc_private = true
```

## Limitations

To run a library's lints on a package, Dylint tries to build the package with the same toolchain used to build the library. So if a package requires a specific toolchain to build, Dylint may not be able to apply certain libraries to that package.

One way this problem can manifest itself is if you try to run one library's lints on the source code of another library. That is, if two libraries use different toolchains, they may not be applicable to each other.

## Resources

Helpful resources for writing lints include the following:

- [Adding a new lint] (targeted at Clippy but still useful)
- [Author lint]
- [Common tools for writing lints]
- [Guide to Rustc Development]
- [Crate `rustc_hir`]
- [Crate `rustc_middle`]
- [Struct `rustc_lint::LateContext`]
  - [Method `typeck_results`]
  - [Field `tcx`]
    - [Method `hir`]

[`clippy_utils`]: https://github.com/rust-lang/rust-clippy/tree/master/clippy_utils
[`compiletest_rs`]: https://github.com/Manishearth/compiletest-rs
[`dylint-link`]: ../dylint-link
[`dylint_library!`]: ../utils/linting
[`env_cargo_path`]: ../examples/general/env_cargo_path
[`latelintpass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html
[`non_thread_safe_call_in_test`]: ../examples/general/non_thread_safe_call_in_test
[`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[`try_io_result`]: ../examples/restriction/try_io_result
[`ui_test`]: ../utils/testing
[`unknown_lints`]: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unknown-lints
[adding a new lint]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/adding_lints.md
[author lint]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/adding_lints.md#author-lint
[common tools for writing lints]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/common_tools_writing_lints.md
[conditional compilation]: #conditional-compilation
[crate `rustc_hir`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/index.html
[crate `rustc_middle`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/index.html
[dylint/src/lib.rs]: ../dylint/src/lib.rs
[example general lints]: ../examples/general
[example lints]: ../examples
[field `tcx`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html#structfield.tcx
[glob]: https://docs.rs/glob/0.3.0/glob/struct.Pattern.html
[guide to rustc development]: https://rustc-dev-guide.rust-lang.org/
[here]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[how libraries are found]: #how-libraries-are-found
[internal/src/examples.rs]: ../internal/src/examples.rs
[library requirements]: #library-requirements
[limitations]: #limitations
[method `hir`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/context/struct.TyCtxt.html#method.hir
[method `typeck_results`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html#method.typeck_results
[quick start]: #quick-start
[resources]: #resources
[running dylint]: #running-dylint
[rust-analyzer]: https://github.com/rust-analyzer/rust-analyzer
[struct `rustc_lint::latecontext`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html
[utilities]: #utilities
[vs code integration]: #vs-code-integration
[workspace metadata]: #workspace-metadata
[writing lints]: #writing-lints
