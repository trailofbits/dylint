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

mod redundant_reference {
    use rustc_hir::intravisit::Visitor;
    use rustc_lint::LateContext;

    struct V<'cx, 'tcx> {
        cx: &'cx LateContext<'tcx>,
    }

    impl<'cx, 'tcx> Visitor<'tcx> for V<'cx, 'tcx> {
        type Map = rustc_middle::hir::map::Map<'tcx>;
        type NestedFilter = rustc_middle::hir::nested_filter::All;

        fn nested_visit_map(&mut self) -> Self::Map {
            self.cx.tcx.hir()
        }
    }
}

mod uncopyable_subfield {
    struct S<'a> {
        bar: &'a super::Bar,
    }

    impl<'a> S<'a> {
        fn foo(&self) -> String {
            self.bar.baz.clone()
        }
    }
}

mod public_struct_and_field {
    pub struct S<'a> {
        pub bar: &'a super::Bar,
    }

    impl<'a> S<'a> {
        fn foo(&self) -> bool {
            self.bar.qux
        }
    }
}

mod mutable_reference {
    struct S<'a> {
        bar: &'a mut super::Bar,
    }

    impl<'a> S<'a> {
        fn foo(&mut self) {
            self.bar.qux = true;
        }
    }
}

mod multiple_copyable_subfield_reads {
    struct S<'a> {
        bar: &'a super::Bar,
    }

    impl<'a> S<'a> {
        fn foo(&self) -> bool {
            self.bar.qux || self.bar.quux
        }
    }
}

mod other_use {
    struct S<'a> {
        bar: &'a super::Bar,
    }

    impl<'a> S<'a> {
        fn foo(&self) -> *const super::Bar {
            self.bar
        }
    }
}
