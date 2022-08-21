// run-rustfix

#![allow(unused_imports)]

use std::{
    borrow::{Borrow, BorrowMut},
    io::Read,
    ops::{Deref, DerefMut},
    path::Path,
    process::Command,
};
use tempfile::{NamedTempFile, TempDir};

fn main() {
    let mut readable = Box::new(&[] as &[u8]);
    let s = String::new();
    let tempdir = TempDir::new().unwrap();
    let tempfile = NamedTempFile::new().unwrap();

    let _ = std::fs::write("x", s.as_bytes());
    let _ = std::fs::write("x", "".as_bytes());
    let _ = std::fs::write("x", "".to_owned());
    let _ = std::fs::write("x", "".to_string());
    let _ = std::fs::write("x", <_ as AsRef<[u8]>>::as_ref(""));
    let _ = std::fs::write("x", <_ as AsRef<str>>::as_ref(""));
    let _ = std::fs::write("x", "".borrow());
    let _ = std::fs::write("x", "".deref());

    read(readable.as_mut());
    read(<_ as BorrowMut<&[u8]>>::borrow_mut(&mut readable));
    read(<_ as BorrowMut<Box<_>>>::borrow_mut(&mut readable));
    read(readable.deref_mut());

    let _ = Command::new("ls")
        .args(["-a", "-l"].iter())
        .status()
        .unwrap();
    let _ = Path::new("/").join(Path::new("."));
    let _ = Path::new("/").join(tempdir.path());
    let _ = Path::new("/").join(tempfile.path());
}

fn read(_: impl Read) {}
