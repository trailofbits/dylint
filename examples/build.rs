use std::env::var_os;

fn main() {
    // smoelius: Don't allow Dylint itself to cause the examples to be built. Currently, Nested
    // Workspace does not clear `RUSTC_WORKSPACE_WRAPPER`. Building the `straggler` library with
    // this environment variable set can cause "incompatible version of rustc" errors.
    if var_os(dylint_internal::env::RUSTC_WORKSPACE_WRAPPER).is_none() {
        nested_workspace::build().unwrap();
    }
}
