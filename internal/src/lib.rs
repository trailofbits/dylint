#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(nonexistent_path_in_comment))]
#![cfg_attr(nightly, feature(rustc_private))]

#[cfg(feature = "cargo")]
pub mod cargo;

#[cfg(feature = "clippy_utils")]
pub mod clippy_utils;

#[cfg(feature = "config")]
pub mod config;

mod command;
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

#[cfg(feature = "home")]
pub mod home;

#[cfg(all(nightly, feature = "match_def_path"))]
mod match_def_path;
#[cfg(all(nightly, feature = "match_def_path"))]
pub use match_def_path::{match_any_def_paths, match_def_path};

pub mod msrv;

#[cfg(feature = "packaging")]
pub mod packaging;

pub mod paths;

#[cfg(feature = "rustup")]
pub mod rustup;

mod sed;
pub use sed::find_and_replace;

#[cfg(feature = "testing")]
pub mod testing;
