# How Dylint works

**Contents**

- [How libraries are found]
- [Library requirements]
- [Limitations]

## How libraries are found

Dylint tries to run all lints in all libraries named on the command line. Dylint resolves names to libraries in the following three ways:

1. Via a `--git` or `--path` option on the command line. If either appears, its url or path (respectively) is treated as part of a [workspace metadata] entry. Note that a `--git` or `--path` option can be accompanied by a `--pattern` option to specify subdirectories containing library packages. Furthermore, a `--git` option can be accompanied by a `--branch`, `--tag`, or `--rev` option.

2. Via the `DYLINT_LIBRARY_PATH` environment variable. If `DYLINT_LIBRARY_PATH` is set when Dylint is started, Dylint treats it as a colon-separated list of paths, and searches each path for files with names of the form `DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX` (see [Library requirements] below). For each such file found, `LIBRARY_NAME` resolves to that file.

3. Via workspace metadata. If Dylint is started in a workspace, Dylint checks the workspace's `Cargo.toml` file for `workspace.metadata.dylint.libraries` (see [Workspace metadata] in the repository's main [README.md]). Dylint downloads and builds each listed entry, similar to how Cargo downloads and builds a dependency. The resulting `target/release` directories are searched and names are resolved in the manner described in 1 above.

4. By path. If a name does not resolve to a library via 1 or 2, it is treated as a path.

It is considered an error if a name used on the command line resolves to multiple libraries.

If `--lib name` is used, then `name` is treated only as a library name, and not as a path.

If `--lib-path name` is used, then `name` is treated only as a path, and not as a library name.

If `--all` is used, Dylint runs all lints in all libraries discovered via 1 and 2 above.

Note: Earlier versions of Dylint searched the current package's `target/debug` and `target/release` directories for libraries. This feature has been removed.

## Library requirements

A Dylint library must satisfy four requirements. **Note:** Before trying to satisfy these explicitly, see [Utilities] in the repository's main [README.md].

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

## Limitations

To run a library's lints on a package, Dylint tries to build the package with the same toolchain used to build the library. So if a package requires a specific toolchain to build, Dylint may not be able to apply certain libraries to that package.

One way this problem can manifest itself is if you try to run one library's lints on the source code of another library. That is, if two libraries use different toolchains, they may not be applicable to each other.

[How libraries are found]: #how-libraries-are-found
[Library requirements]: #library-requirements
[Limitations]: #limitations
[README.md]: ../README.md
[Utilities]: ../README.md#utilities
[Workspace metadata]: ../README.md#workspace-metadata
[`dylint-link`]: ../dylint-link
[`dylint_library!`]: ../utils/linting
[`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[here]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[utilities]: ../README.md#utilities
[workspace metadata]: ../README.md#workspace-metadata
