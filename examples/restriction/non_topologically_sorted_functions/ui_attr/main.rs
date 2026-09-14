fn main() {
    foo();
}

// smoelius: False negative.
#[attr::attr]
fn bar() {}

fn foo() {
    bar();
}
