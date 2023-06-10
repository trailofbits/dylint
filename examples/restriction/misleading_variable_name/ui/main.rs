#![allow(unused)]

use anyhow::{Context, Result};
use std::{
    fs::read_to_string,
    io::{BufRead, Cursor},
    path::Path,
};

mod one_type {
    pub struct Bar;

    pub fn foo() -> bool {
        false
    }
}

mod two_types {
    pub struct Bar;
    pub struct Baz;

    pub fn foo() -> bool {
        false
    }
}

mod private {
    struct Bar;

    pub fn foo() -> bool {
        false
    }
}

mod rename {
    pub use std::process::Command as Bar;

    pub fn foo() -> Bar {
        Bar::new("true")
    }
}

mod rc {
    pub struct Bar;

    pub fn foo() -> std::rc::Rc<Bar> {
        std::rc::Rc::new(Bar)
    }
}

fn main() -> Result<()> {
    let path = Path::new("x");

    let file = read_to_string(path)?;
    let file = read_to_string(path).with_context(|| "read")?;
    let file = read_to_string(path).unwrap();

    let buf_reader = Cursor::new([]).lines();

    let bar = one_type::foo();

    let bar = two_types::foo();

    // negative tests
    let contents = read_to_string(path).unwrap();

    let lines = Cursor::new([]).lines();

    let bar = private::foo();

    // private in extern crate
    let command = cargo_metadata::MetadataCommand::new();

    let bar = rename::foo();

    let bar = rc::foo();

    let rc = std::rc::Rc::new(String::new());

    Ok(())
}

// smoelius: Don't flag functions/types defined in the same module as the call.
mod same_mod {
    pub struct Bar;

    pub fn foo() -> bool {
        false
    }

    fn baz() {
        let bar = foo();
    }
}
