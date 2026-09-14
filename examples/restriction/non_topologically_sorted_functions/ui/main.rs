fn main() {
    foo();
}

fn foo() {
    bar();
}

fn bar() {
    let t_struct = TestStruct;
}

struct TestStruct;

// negative test (test functions should be ignored)
#[test]
fn baz() {
    foo();
}
