#![allow(clippy::missing_const_for_fn, clippy::panic)]
#![cfg_attr(dylint_lib = "crate_wide_allow", allow(crate_wide_allow))]

fn main() {}

#[test]
fn panic() {
    if false {
        panic!();
    }
}
