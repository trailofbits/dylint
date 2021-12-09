pub mod cargo;
pub use cargo::{build, check, fix, test};

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

#[cfg(feature = "git2")]
pub mod packaging;

pub mod paths;

pub mod rustup;

mod sed;
pub use sed::find_and_replace;

#[cfg(feature = "git2")]
mod testing;
#[cfg(feature = "git2")]
pub use testing::clone_dylint_template;
