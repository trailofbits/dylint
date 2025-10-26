fn main() {
    foo();
}

fn baz() {}

fn foo() {
    bar();
    {
        baz();
    }
}

fn bar() {}
