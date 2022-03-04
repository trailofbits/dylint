#![allow(unknown_lints)]
#![warn(allow_clippy)]

#[allow(clippy::assertions_on_constants)]
fn main() {
    assert!(true);
}

mod inner_attribute {
    #![allow(clippy::assertions_on_constants)]
    fn foo() {}
    fn bar() {
        assert!(true);
    }
}
