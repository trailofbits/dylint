// run-rustfix
#![expect(dead_code)]

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

fn non_const_vec(x: Vec<u32>) {
    assert_eq!(x, vec![0, 1, 2]);
}

fn non_const_vec_repeat(x: Vec<u32>) {
    assert_eq!(x, vec![0; 3]);
}

// `assert_eq!(vec![], x)` would not compile.
fn non_const_vec_empty(x: Vec<u32>) {
    assert_eq!(x, vec![]);
}

fn non_const_vec_multiline(vec_with_a_really_long_name: Vec<u32>) {
    assert_eq!(
        vec_with_a_really_long_name,
        vec![CONST_WITH_A_REALLY_LONG_NAME, CONST_WITH_A_REALLY_LONG_NAME]
    );
}

fn non_const_vec_of_non_const(x: Vec<u32>, y: u32) {
    assert_eq!(x, vec![y]);
}

fn vec_vec() {
    assert_eq!(vec![0], vec![1]);
}
