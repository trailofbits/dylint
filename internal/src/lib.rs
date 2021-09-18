pub mod cargo;
pub use cargo::{build, check, test};

mod command;
pub use command::*;

pub mod env;

pub mod examples;

mod filename;
pub use filename::library_filename;

#[cfg(feature = "git2")]
mod git;
#[cfg(feature = "git2")]
pub use git::*;

pub mod path;

pub mod rustup;

mod sed;
pub use sed::find_and_replace;

#[cfg(feature = "git2")]
pub mod testing;
#[cfg(feature = "git2")]
pub use testing::checkout_dylint_template;
