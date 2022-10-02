pub const DYLINT_VERSION: &str = "0.1.0";

pub use paste;

// smoelius: Including `extern crate rustc_driver` causes the library to link against
// `librustc_driver.so`, which dylint-driver also links against. So, essentially, the library uses
// dylint-driver's copy of the Rust compiler crates.
#[macro_export]
macro_rules! dylint_library {
    () => {
        #[allow(unused_extern_crates)]
        extern crate rustc_driver;

        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
            std::ffi::CString::new($crate::DYLINT_VERSION)
                .unwrap()
                .into_raw()
        }
    };
}

#[macro_export]
macro_rules! __declare_and_register_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $register_pass_method:ident, $pass:expr) => {
        $crate::dylint_library!();

        extern crate rustc_lint;
        extern crate rustc_session;

        #[no_mangle]
        pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
            lint_store.register_lints(&[$NAME]);
            lint_store.$register_pass_method($pass);
        }

        rustc_session::declare_lint!($(#[$attr])* $vis $NAME, $Level, $desc);
    };
}

#[rustversion::before(2022-09-08)]
#[macro_export]
macro_rules! __make_late_closure {
    ($pass:expr) => {
        || Box::new($pass)
    };
}

// Relevant PR and merge commit:
// * https://github.com/rust-lang/rust/pull/101501
// * https://github.com/rust-lang/rust/commit/87788097b776f8e3662f76627944230684b671bd
#[rustversion::since(2022-09-08)]
#[macro_export]
macro_rules! __make_late_closure {
    ($pass:expr) => {
        |_| Box::new($pass)
    };
}

#[macro_export]
macro_rules! impl_pre_expansion_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_pre_expansion_pass,
            || Box::new($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! impl_early_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_early_pass,
            || Box::new($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! impl_late_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr, $pass:expr) => {
        $crate::__declare_and_register_lint!(
            $(#[$attr])* $vis $NAME,
            $Level,
            $desc,
            register_late_pass,
            $crate::__make_late_closure!($pass)
        );
        $crate::paste::paste! {
            rustc_session::impl_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_pre_expansion_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_pre_expansion_pass,
                || Box::new([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_early_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_early_pass,
                || Box::new([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}

#[macro_export]
macro_rules! declare_late_lint {
    ($(#[$attr:meta])* $vis:vis $NAME:ident, $Level:ident, $desc:expr) => {
        $crate::paste::paste! {
            $crate::__declare_and_register_lint!(
                $(#[$attr])* $vis $NAME,
                $Level,
                $desc,
                register_late_pass,
                $crate::__make_late_closure!([< $NAME:camel >])
            );
            rustc_session::declare_lint_pass!([< $NAME:camel >] => [$NAME]);
        }
    };
}
