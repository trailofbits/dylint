#![feature(register_tool)]
#![register_tool(dylint)]
#![warn(dylint::allow_clippy)]

#[allow(clippy::assertions_on_constants)]
fn main() {
    assert!(true);
}
