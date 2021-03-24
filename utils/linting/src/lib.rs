pub const DYLINT_VERSION: &str = env!("CARGO_PKG_VERSION");

// smoelius: Including `extern crate rustc_driver` causes the library to link against
// `librustc_driver.so`, which dylint-driver also links against. So, essentially, the library uses
// dylint-driver's copy of the Rust compiler crates.
#[macro_export]
macro_rules! dylint_library {
    () => {
        #[allow(unused_extern_crates)]
        extern crate rustc_driver;

        #[no_mangle]
        pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
            std::ffi::CString::new($crate::DYLINT_VERSION)
                .unwrap()
                .into_raw()
        }
    };
}
