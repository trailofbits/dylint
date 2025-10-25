fn main() {}

fn foo() {
    bar();
    baz();
}

fn qux() {
    baz();
    bar();
}

fn bar() {}

fn baz() {}
