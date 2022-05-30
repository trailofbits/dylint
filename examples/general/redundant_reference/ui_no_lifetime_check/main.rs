#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;

pub struct Bar {
    baz: String,
    qux: bool,
    quux: bool,
}

fn main() {}

// smoelius: For some reason, the order in which the diagnostic messages are printed varies when
// `REDUNDANT_REFERENCE_NO_LIFETIME_CHECK` is enabled. The next module is the only one that produces
// an additional warning when the feature is enabled. So, the work around is to ensure that this
// module appears first (to facilitate comparing files), and that its warning is the only one
// produced when `REDUNDANT_REFERENCE_NO_LIFETIME_CHECK` is enabled.
mod multiple_lifetime_uses {
    struct S<'a> {
        bar: &'a super::Bar,
        baz: &'a str,
    }

    impl<'a> S<'a> {
        fn foo(&self) -> bool {
            self.bar.qux
        }
    }
}
