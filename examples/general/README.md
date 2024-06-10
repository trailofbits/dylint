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

[`constituent` feature]: ../../utils/linting/README.md#constituent-feature
