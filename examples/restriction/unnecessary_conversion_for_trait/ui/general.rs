// run-rustfix

#![allow(unused_imports, unused_parens)]
#![feature(path_as_mut_os_str)]

use std::{
    borrow::{Borrow, BorrowMut},
    ffi::{OsStr, OsString},
    io::Read,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{NamedTempFile, TempDir};

fn main() {
    let mut readable = Box::new(&[] as &[u8]);
    let mut s = String::new();
    let mut vec = Vec::<u8>::new();
    let mut path_buf = PathBuf::from("x");
    let osstr = OsStr::new("");
    let osstring = OsString::new();
    let path = Path::new("x");
    let tempdir = TempDir::new().unwrap();
    let tempfile = NamedTempFile::new().unwrap();

    // trait methods

    let _ = std::fs::write("x", "".to_owned());

    let _ = std::fs::write("x", "".to_string());

    let _ = std::fs::write("x", "".borrow());

    read(<_ as BorrowMut<&[u8]>>::borrow_mut(&mut readable));
    read(<_ as BorrowMut<Box<_>>>::borrow_mut(&mut readable));

    read(readable.as_mut());

    let _ = std::fs::write("x", <_ as AsRef<[u8]>>::as_ref(""));
    let _ = std::fs::write("x", <_ as AsRef<str>>::as_ref(""));

    let _ = std::fs::write("x", "".deref());

    read(readable.deref_mut());

    // inherent methods

    let _ = std::fs::write("x", (Box::new([]) as Box<[u8]>).into_vec());
    let _ = std::fs::write("x", (&[] as &[u8]).to_vec());

    let _ = is_empty(s.clone().into_boxed_str().into_boxed_bytes());
    let _ = is_empty(s.clone().into_boxed_str().into_string());

    let _ = std::fs::write("x", s.as_bytes());
    let _ = std::fs::write("x", s.as_mut_str());
    let _ = std::fs::write("x", s.as_str());
    let _ = is_empty(s.clone().into_boxed_str());
    let _ = std::fs::write("x", s.clone().into_bytes());

    let _ = std::fs::write("x", vec.as_mut_slice());
    let _ = std::fs::write("x", vec.as_slice());
    let _ = std::fs::write("x", vec.into_boxed_slice());

    let _ = Command::new("ls").args(["-a", "-l"].iter());
    let _ = Command::new("ls").args(["-a", "-l"].iter_mut());

    let _ = std::fs::write("x", "".as_bytes());

    let _ = is_empty_os(osstring.clone().into_boxed_os_str().into_os_string());
    let _ = std::fs::write(OsStr::new("x"), "");
    let _ = std::fs::write(osstr.to_os_string(), "");

    let _ = std::fs::write(osstring.as_os_str(), "");
    let _ = is_empty_os(osstring.clone().into_boxed_os_str());

    let _ = std::fs::write(path.as_os_str(), "");
    let _ = std::fs::write(PathBuf::from("x").as_mut_os_str(), "");
    let _ = std::fs::write(PathBuf::from("x").into_boxed_path().into_path_buf(), "");
    let _ = Command::new("ls").args(path.iter());
    let _ = std::fs::write(Path::new("x"), "");
    let _ = std::fs::write(path.to_path_buf(), "");

    let _ = std::fs::write(path_buf.as_mut_os_string(), "");
    let _ = std::fs::write(path_buf.as_path(), "");
    let _ = std::fs::write(path_buf.clone().into_os_string(), "");

    let _ = std::fs::write(tempdir.path(), "");
    let _ = std::fs::write(tempfile.path(), "");
}

fn read(_: impl Read) {}

#[must_use]
fn is_empty<T: From<Box<str>> + PartialEq>(x: T) -> bool {
    x == T::from(String::new().into_boxed_str())
}

#[must_use]
fn is_empty_os<T: From<Box<OsStr>> + PartialEq>(x: T) -> bool {
    x == T::from(OsString::new().into_boxed_os_str())
}
