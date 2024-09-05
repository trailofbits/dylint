// run-rustfix

fn main() {}

// A comment.
#[expect(clippy::disallowed_names)]
pub fn foo() {}

/// Negative test.
pub fn bar() {}
