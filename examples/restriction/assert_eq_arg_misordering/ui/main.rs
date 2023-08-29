// run-rustfix
#![allow(dead_code)]

fn main() {}

const CONST_WITH_A_REALLY_LONG_NAME: u32 = 0;

fn non_const_const(x: u32) {
    assert_eq!(x, 0);
}

fn non_const_const_multiline(variable_with_a_really_long_name: u32) {
    assert_eq!(
        variable_with_a_really_long_name,
        CONST_WITH_A_REALLY_LONG_NAME
    );
}

fn non_const_const_with_message(x: u32) {
    assert_eq!(x, 0, "this is a message (with parens)");
}

fn const_const() {
    assert_eq!(0, 0);
}

fn non_const_non_const(x: u32, y: u32) {
    assert_eq!(x, y);
}
