# General-purpose lints

## Use of `dylint_linting`'s `constituent` feature

The general-purpose lints use `dylint_linting`'s [`constituent` feature]. This allows the lints to be built in either of the following two configurations:

- as individual libraries
- as a single, combined `general` library

However, some additional organization of both the `general` directory and its subdirectories is required.

For the general directory itself:

- Each lint is listed under `[dependencies]` in general/Cargo.toml
- Each lint is listed under `[workspace.members]` in general/Cargo.toml
- Each lint's `register_lints` function is called from the `register_lints` function in src/lib.rs.

For each lint subdirectory:

- The following files do not appear, even though they would be created by `cargo dylint new`:
  - .cargo/config.toml
  - .gitignore
  - rust-toolchain
- The lint's Cargo.toml has the following `[lib]` section (note: `crate-type` is not just `["cdylib"]`):
  ```toml
  [lib]
  crate-type = ["cdylib", "rlib"]
  ```
- The lint gets its `clippy_utils` dependency from the workspace, i.e.:
  ```toml
  [dependencies]
  clippy_utils = { workspace = true }
  ```
- The lint's Cargo.toml has the following `[features]` section:
  ```toml
  [features]
  rlib = ["dylint_linting/constituent"]
  ```


### abs_home_path

Checks for absolute paths that contain `/home/`.

### arg_iter

Checks for functions that take `Iterator` trait bounds when they could use `IntoIterator` instead.

This lint encourages using more flexible function signatures that can accept collections directly (like slices, vectors, arrays) without requiring explicit `.iter()` or `.into_iter()` calls.

### await_holding_span_guard

Detects calls to `.await` while holding a [Span::enter guard].


### basic_dead_store

Suggests using underscore-prefixed variable names for variables that are written but never read.

### crate_wide_allow

Detects crate-wide lint allows.

### incorrect_matches_operation

Detects attempts to use a `matches!(...)` expression as a full pattern match (which can lead to missed cases) instead of `if let`/`match`.

### non_local_effect_before_error_return

Detects functions that have non-local effects before error return.

Non-local effects include:
1. writes to fields of 'self', struct, or arrays/slices/vectors
2. calls to functions that might have non-local effects (excluding debug formatting 'write!', 'println!', etc. or debug assertions)

By avoiding non-local effects before early returns with errors, the change makes failure safer: the state is modified if and only if the operation succeeds as a whole.

### non_thread_safe_call_in_test

Detects direct use of non-thread-safe rust test features, such as:
1. `set_var` / `remove_var` from std::env,
   which are not thread-safe.
2. std::env::set_current_dir,
   which changes the process-wide working directory.

Note: This is similar to the Clippy lint `non_thread_safe_call_in_test`, but the implementations differ.

### wrong_serialize_struct_arg

Warns on use of `serialize_struct` with a wrong capacity N, when it expects N fields but gets more or
less.

[Span::enter guard]: https://docs.rs/tracing/latest/tracing/struct.Span.html#method.enter

[Span::enter guard]: https://docs.rs/tracing/latest/tracing/struct.Span.html#method.enter
[`constituent` feature]: ../../utils/linting/README.md#constituent-feature
