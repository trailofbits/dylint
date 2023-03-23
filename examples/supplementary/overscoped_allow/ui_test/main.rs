#![allow(clippy::missing_const_for_fn, clippy::manual_assert)]
#![cfg_attr(dylint_lib = "crate_wide_allow", allow(crate_wide_allow))]
#![warn(clippy::panic)]

fn main() {}

#[allow(clippy::panic)]
mod outside {
    #[test]
    fn panic() {
        if false {
            panic!();
        }
    }
}

mod inside {
    #[test]
    fn panic() {
        #[allow(clippy::panic)]
        if false {
            panic!();
        }
    }
}
