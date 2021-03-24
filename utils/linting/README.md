# dylint_linting

This crate provides a `dylint_library!` macro to help in creating [Dylint](https://github.com/trailofbits/dylint) libraries.

The macro expands to the following:

```rust
#[allow(unused_extern_crates)]
extern crate rustc_driver;

#[no_mangle]
pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
    std::ffi::CString::new($crate::DYLINT_VERSION)
        .unwrap()
        .into_raw()
}
```

If your library uses the `dylint_library!` macro and the [`dylint-link`](../../dylint-link) tool, then all you should have to do is implement the [`register_lints`](https://doc.rust-lang.org/stable/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints) function. See the [examples](../../examples) in this repository.
