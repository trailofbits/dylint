pub mod cargo;
pub use cargo::{build, check, test};

mod command;
pub use command::*;

pub mod env;

pub mod examples;

mod git;
pub use git::*;

pub mod rustup;

pub mod testing;
pub use testing::checkout_dylint_template;
