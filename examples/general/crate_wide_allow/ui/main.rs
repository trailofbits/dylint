#![allow(clippy::assertions_on_constants)]

fn main() {
    assert!(true);
}

mod inner_attribute {
    #![allow(clippy::bool_assert_comparison)]
    #![allow(dead_code)]
    fn foo() {}
    fn bar() {
        assert_eq!("a".is_empty(), false);
    }
}
