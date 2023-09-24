# marker

### What it does
Runs Marker lints from a Dylint library.

### Configuration
- `lint_crates`: A list of [`marker_adapter::LintCrateInfo`]. Each is a struct containing
  two fields, `name` and `path`, which are documented as follows:
  - `name`: The name of the lint crate
  - `path`: The absolute path of the compiled dynamic library, which can be loaded as a lint
    crate

[`marker_adapter::LintCrateInfo`]: https://docs.rs/marker_adapter/latest/marker_adapter/struct.LintCrateInfo.html
