#[cfg(feature = "cargo")]
pub mod cargo;

#[cfg(feature = "command")]
mod command;
#[cfg(feature = "command")]
pub use command::*;

pub mod env;

#[cfg(feature = "examples")]
pub mod examples;

mod filename;
pub use filename::{library_filename, parse_path_filename};

#[cfg(feature = "git")]
mod git;
#[cfg(feature = "git")]
pub use git::*;

#[cfg(feature = "git")]
pub use git2;

#[cfg(feature = "packaging")]
pub mod packaging;

pub mod paths;

#[cfg(feature = "rustup")]
pub mod rustup;

#[cfg(feature = "sed")]
mod sed;
#[cfg(feature = "sed")]
pub use sed::find_and_replace;

#[cfg(feature = "testing")]
pub mod testing;
