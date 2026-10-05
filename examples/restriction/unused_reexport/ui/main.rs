#![allow(dead_code, unused_imports)]

fn main() {
    facade::marker();
    let _ = origin::First;
    let _ = origin::Second;
    let _ = origin::nested::Third;
    let _ = origin::Private;
}

mod imported {
    fn before() {
        let _ = origin::First;
        let _ = origin::Second;
    }
    use origin::First;
    fn after() {
        let _ = origin::First;
    }
    mod child {
        fn check() {
            let _ = origin::First;
        }
    }
}

#[cfg(any())]
fn inactive() {
    let _ = origin::First;
}

macro_rules! generated {
    () => {
        fn generated() {
            let _ = origin::First;
        }
    };
}
generated!();

mod shadowed {
    mod origin {
        pub struct First;
    }
    fn check() {
        let _ = origin::First;
    }
}

mod grouped {
    use origin::{First, Second as Other};
    fn check() {
        let _ = origin::First;
        let _ = Other;
    }
}

mod renamed {
    use origin::Second;
    fn check() {
        let _ = Second;
    }
}

#[cfg_attr(dylint_lib = "unused_reexport", allow(unused_reexport))]
mod allowed {
    fn check() {
        let _ = origin::First;
    }
}

fn more_paths() {
    let _ = ::origin::Generic::<u8>(0);
    let _: Option<origin::Generic<u8>> = None;
    let _ = origin::Lookalike;
    let _ = origin::r#type::Keyword;
}

mod aliased_import {
    use origin::Second as Local;
    fn check() {
        let _ = Local;
    }
}

mod allowed_import {
    #[cfg_attr(dylint_lib = "unused_reexport", allow(unused_reexport))]
    use origin::First;
    fn check() {
        let _ = origin::First;
    }
}
