# Dylint

Run Rust lints from dynamic libraries (EuroRust 2024 [slides] and [video])

```sh
cargo install cargo-dylint dylint-link
```

Dylint is a Rust linting tool, similar to Clippy. But whereas Clippy runs a predetermined, static set of lints, Dylint runs lints from user-specified, dynamic libraries. Thus, Dylint allows developers to maintain their own personal lint collections.

**Contents**

- [Quick start]
  - [Running Dylint]
  - [Writing lints]
- [Features]
  - [Workspace metadata]
  - [Configurable libraries]
  - [Conditional compilation]
  - [VS Code integration]
- [Utilities]
- [Resources]
- [MSRV policy]

Documentation is also available on [how Dylint works].

## Quick start

### Running Dylint

The next two steps install Dylint and run all of this repository's [general-purpose, example lints] on a workspace:

1. Install `cargo-dylint` and `dylint-link`:

   ```sh
   cargo install cargo-dylint dylint-link
   ```

2. Run `cargo-dylint`:
   ```sh
   cargo dylint --git https://github.com/trailofbits/dylint --pattern examples/general
   ```

In the above example, the libraries are found via the command line. If you plan to run Dylint regularly, then consider using [workspace metadata]. For additional ways of finding libraries, see [How Dylint works].

### Writing lints

You can start writing your own Dylint library by running `cargo dylint new new_lint_name`. Doing so will produce a loadable library right out of the box. You can verify this as follows:

```sh
cargo dylint new new_lint_name
cd new_lint_name
cargo build
cargo dylint list --path .
```

All you have to do is implement the [`LateLintPass`] trait and accommodate the symbols asking to be filled in.

Helpful [resources] for writing lints appear below.

## Features

### Workspace metadata

A workspace can name the libraries it should be linted with in its `Cargo.toml` or `dylint.toml` file. Specifically, either file can contain a TOML array under `workspace.metadata.dylint.libraries`. Each array entry must have the form of a Cargo `git` or `path` dependency, with the following differences:

- There is no leading package name, i.e., no `package =`.
- `path` entries can contain [glob] patterns, e.g., `*`.
- Any entry can contain a `pattern` field whose value is a [glob] pattern. The `pattern` field indicates the subdirectories that contain Dylint libraries.

Dylint downloads and builds each entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched for files with names of the form that Dylint recognizes (see [Library requirements] under [How Dylint works]).

As an example, if you include the following in your workspace's `Cargo.toml` or `dylint.toml` file and run `cargo dylint --all`, Dylint will run all of this repository's [example general-purpose lints], as well as the example restriction lint [`try_io_result`].

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/trailofbits/dylint", pattern = "examples/general" },
    { git = "https://github.com/trailofbits/dylint", pattern = "examples/restriction/try_io_result" },
]
```

For convenience, the `pattern` field can contain an array, in which case the pattern is considered to be the union of the array elements. Thus, the just given `workspace.metadata.dylint.libraries` example could alternatively be written as:

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/trailofbits/dylint", pattern = [
        "examples/general",
        "examples/restriction/try_io_result",
    ] },
]
```

The `git` field can be accompanied by a `branch`, `tag`, or `rev` option.

```toml
[workspace.metadata.dylint]
libraries = [
    # All of these are valid ways to load lints from a remote git repository
    { git = "https://github.com/trailofbits/dylint", tag = "v5.0.0", pattern = "examples/general" },
    { git = "https://github.com/trailofbits/dylint", branch = "master", pattern = "examples/general" },
    { git = "https://github.com/trailofbits/dylint", rev = "76b73b33dffa2505ad179bd5fce0134a90a055e4", pattern = "examples/general" }
]
```

### Configurable libraries

Libraries can be configured by including a `dylint.toml` file in a linted workspace's root directory. The file should encode a [toml table] whose keys are library names. A library determines how its value in the table (if any) is interpreted.

As an example, a `dylint.toml` file with the following contents sets the [`non_local_effect_before_error_return`] library's `work_limit` configuration to `1_000_000`:

```toml
[non_local_effect_before_error_return]
work_limit = 1_000_000
```

For instructions on creating a configurable library, see the [`dylint_linting`] documentation.

### Conditional compilation

For each library that Dylint uses to check a crate, Dylint passes the following to the Rust compiler:

```sh
--cfg=dylint_lib="LIBRARY_NAME"
```

You can use this feature to allow a lint when Dylint is used, but also avoid an "unknown lint" warning when Dylint is not used. Specifically, you can do the following:

```rust
#[cfg_attr(dylint_lib = "LIBRARY_NAME", allow(LINT_NAME))]
```

Note that `LIBRARY_NAME` and `LINT_NAME` may be the same. For an example involving [`non_thread_safe_call_in_test`], see [dylint/src/lib.rs] in this repository.

Also note that the just described approach does not work for pre-expansion lints. The only known workaround for pre-expansion lints is to allow the compiler's built-in [`unknown_lints`] lint. Specifically, you can do the following:

```rust
#[allow(unknown_lints)]
#[allow(PRE_EXPANSION_LINT_NAME)]
```

For an example involving [`abs_home_path`], see [internal/src/examples.rs] in this repository.

#### Rustc's `unexpected_cfg` lint

As of nightly-2024-05-05, the names and values of every reachable `#[cfg]` [are checked]. This causes the compiler to produce warnings for `cfg_attr` attributes as described above.

To suppress such warnings, add the following to your packages' Cargo.toml files:

```toml
[lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

Or, if you're using a Cargo workspace, add the following the workspace's Cargo.toml file:

```toml
[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

Then, add the following to the Cargo.toml file of each package in the workspace:

```toml
[lints]
workspace = true
```

For an example, see commit [`bc16236`] in this repository.

### VS Code integration

Dylint results can be viewed in VS Code using [rust-analyzer]. To do so, add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.check.overrideCommand": [
        "cargo",
        "dylint",
        "--all",
        "--",
        "--all-targets",
        "--message-format=json"
    ]
```

If you want to use rust-analyzer inside a lint library, you need to add the following to your VS Code `settings.json` file:

```json
    "rust-analyzer.rustc.source": "discover",
```

And add the following to the library's `Cargo.toml` file:

```toml
[package.metadata.rust-analyzer]
rustc_private = true
```

## Utilities

The following utilities can be helpful for writing Dylint libraries:

- [`dylint-link`] is a wrapper around Rust's default linker (`cc`) that creates a copy of your library with a filename that Dylint recognizes.
- [`dylint_library!`] is a macro that automatically defines the `dylint_version` function and adds the `extern crate rustc_driver` declaration.
- [`ui_test`] is a function that can be used to test Dylint libraries. It provides convenient access to the [`compiletest_rs`] package.
- [`clippy_utils`] is a collection of utilities to make writing lints easier. It is generously made public by the Rust Clippy Developers. Note that, like `rustc`, `clippy_utils` provides no stability guarantees for its APIs.

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

## MSRV policy

A bump of the Dylint library's MSRV will be accompanied by a bump of at least Dylint's minor version.

Put another way, we strive to preserve Dylint's MSRV when releasing bug fixes, and to change it only when releasing new features.

[Test coverage]

[Adding a new lint]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/adding_lints.md
[Author lint]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/adding_lints.md#author-lint
[Common tools for writing lints]: https://github.com/rust-lang/rust-clippy/blob/master/book/src/development/common_tools_writing_lints.md
[Conditional compilation]: #conditional-compilation
[Configurable libraries]: #configurable-libraries
[Crate `rustc_hir`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/index.html
[Crate `rustc_middle`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/index.html
[Features]: #features
[Field `tcx`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html#structfield.tcx
[Guide to Rustc Development]: https://rustc-dev-guide.rust-lang.org/
[How Dylint works]: ./docs/how_dylint_works.md
[Library requirements]: ./docs/how_dylint_works.md#library-requirements
[MSRV policy]: #msrv-policy
[Method `hir`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/context/struct.TyCtxt.html#method.hir
[Method `typeck_results`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html#method.typeck_results
[Quick start]: #quick-start
[Resources]: #resources
[Running Dylint]: #running-dylint
[Struct `rustc_lint::LateContext`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/struct.LateContext.html
[Test coverage]: https://trailofbits.github.io/dylint/coverage/index.html
[Utilities]: #utilities
[VS Code integration]: #vs-code-integration
[Workspace metadata]: #workspace-metadata
[Writing lints]: #writing-lints
[`LateLintPass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html
[`abs_home_path`]: ./examples/general/abs_home_path
[`bc16236`]: https://github.com/trailofbits/dylint/commit/bc16236fb34c3f98139d2dad469e6a7de179d68d
[`clippy_utils`]: https://github.com/rust-lang/rust-clippy/tree/master/clippy_utils
[`compiletest_rs`]: https://github.com/Manishearth/compiletest-rs
[`dylint-link`]: ./dylint-link
[`dylint_library!`]: ./utils/linting
[`dylint_linting`]: ./utils/linting
[`non_local_effect_before_error_return`]: ./examples/general/non_local_effect_before_error_return
[`non_thread_safe_call_in_test`]: ./examples/general/non_thread_safe_call_in_test
[`try_io_result`]: ./examples/restriction/try_io_result
[`ui_test`]: ./utils/testing
[`unknown_lints`]: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unknown-lints
[are checked]: https://blog.rust-lang.org/2024/05/06/check-cfg.html
[dylint/src/lib.rs]: ./dylint/src/lib.rs
[example general-purpose lints]: ./examples/general
[general-purpose, example lints]: ./examples/README.md#general
[glob]: https://docs.rs/glob/0.3.0/glob/struct.Pattern.html
[how Dylint works]: ./docs/how_dylint_works.md
[internal/src/examples.rs]: ./internal/src/examples.rs
[resources]: #resources
[rust-analyzer]: https://github.com/rust-analyzer/rust-analyzer
[slides]: ./docs/2024-10-11%20Linting%20with%20Dylint%20(EuroRust).pdf
[toml table]: https://toml.io/en/v1.0.0#table
[video]: https://www.youtube.com/watch?v=MjlPUA7sAmA
[workspace metadata]: #workspace-metadata
