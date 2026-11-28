fn main() {
    foo();
}

// False negative: `bar` has an attribute macro and appears before its caller, `foo`.
#[attr::attr]
fn bar() {}

fn foo() {
    bar();
}
