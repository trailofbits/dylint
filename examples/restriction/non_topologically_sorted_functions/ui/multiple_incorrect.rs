fn main() {}

fn foo() {
    bar();
    baz();
}

fn qux() {
    baz();
    bar();
}

fn baz() {}

fn bar() {}
