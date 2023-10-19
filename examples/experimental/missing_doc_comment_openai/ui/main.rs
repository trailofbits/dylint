// run-rustfix

fn main() {}

// A comment.
#[allow(clippy::disallowed_names)]
pub fn foo() {}

/// Negative test.
pub fn bar() {}
