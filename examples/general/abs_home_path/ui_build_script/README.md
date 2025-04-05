# Build Script Test Package

This package is used to test that the `abs_home_path` lint correctly ignores build scripts.

The build script (`build.rs`) intentionally uses `env!("CARGO_MANIFEST_DIR")` which would normally trigger the lint,
but it should be allowed in build scripts as they often need to reference absolute paths.

This test package is used by the `build_script_allowance` test in the `abs_home_path` lint library. 