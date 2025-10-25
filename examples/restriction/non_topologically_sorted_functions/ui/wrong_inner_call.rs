fn main() {
    foo();
}

fn baz() {}

fn foo() {
    bar();
    // This nesting is necessary as part of the logic check.
    // It is checked that nesting will not affect the rule.
    {
        baz();
    }
}

fn bar() {}
