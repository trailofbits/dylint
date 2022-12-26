#![allow(dead_code)]

use std::env::var;

fn main() {
    assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
}

mod module {
    use std::env::var;

    fn foo() {
        assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
    }
}

mod use_self {
    use std::env::{self, var};

    fn foo() {
        assert_eq!(var("LD_PRELOAD"), Err(env::VarError::NotPresent));
    }
}

mod use_glob {
    use std::env::*;

    fn foo() {
        assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
    }
}

mod nested_scopes {
    use std::env::var;

    fn foo() {
        use std::env::var_os;

        assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
        assert_eq!(var_os("LD_PRELOAD"), None);
    }
}

mod use_mod {
    use std::env;

    fn foo() {
        assert_eq!(env::var("LD_PRELOAD"), Err(env::VarError::NotPresent));
    }
}

mod trait_import {
    use std::io::Write;

    fn foo() {
        write!(std::io::sink(), "x").unwrap();
    }
}
