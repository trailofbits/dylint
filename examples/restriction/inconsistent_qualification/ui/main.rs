#![expect(dead_code)]

use std::env::var;

fn main() {
    assert_eq!(Err(std::env::VarError::NotPresent), var("LD_PRELOAD"));
}

mod module {
    use std::env::var;

    fn foo() {
        assert_eq!(Err(std::env::VarError::NotPresent), var("LD_PRELOAD"));
    }
}

mod use_self {
    use std::env::{self, var};

    fn foo() {
        assert_eq!(Err(env::VarError::NotPresent), var("LD_PRELOAD"));
    }
}

mod use_glob {
    use std::env::*;

    fn foo() {
        assert_eq!(Err(std::env::VarError::NotPresent), var("LD_PRELOAD"));
    }
}

mod nested_scopes {
    use std::env::var;

    fn foo() {
        use std::env::var_os;

        assert_eq!(Err(std::env::VarError::NotPresent), var("LD_PRELOAD"));
        assert_eq!(None, var_os("LD_PRELOAD"));
    }
}

mod use_mod {
    use std::env;

    fn foo() {
        assert_eq!(Err(env::VarError::NotPresent), env::var("LD_PRELOAD"));
    }
}

mod trait_import {
    use std::io::Write;

    fn foo() {
        write!(std::io::sink(), "x").unwrap();
    }
}

// smoelius: The lint triggers on `diesel::expression::SelectableExpression`. My current best guess
// is that the compiler is not returning the correct span for that path.
// smoelius: With diesel 2.3, the lint no longer triggers.
mod diesel {
    use diesel::table;

    table! {
        users {
            id -> Integer,
        }
    }
}

mod local_path_false_negative {
    use bar::Baz;

    fn foo() -> Baz {
        bar::Baz::new()
    }

    mod bar {
        pub struct Baz;

        impl Baz {
            pub fn new() -> Self {
                Self
            }
        }
    }
}

mod trait_path {
    use std::borrow::Borrow;

    fn foo<T>(x: &impl Borrow<T>) -> &T {
        <_ as std::borrow::Borrow<T>>::borrow(x)
    }
}

mod relative_module_path {
    #[expect(unused_imports)]
    use bar::baz;

    fn foo() {
        bar::baz::qux()
    }

    mod bar {
        pub mod baz {
            pub fn qux() {}
        }
    }
}

mod underscore_import {
    use baz::Qux as _;

    struct Foo;

    impl baz::Qux for Foo {}

    fn bar() {
        Foo.qux();
    }

    mod baz {
        pub trait Qux {
            fn qux(&self) {}
        }
    }
}

macro_rules! check_ld_preload {
    () => {
        assert_eq!(Err(std::env::VarError::NotPresent), var("LD_PRELOAD"));
    };
}

fn foo() {
    check_ld_preload!();
}
