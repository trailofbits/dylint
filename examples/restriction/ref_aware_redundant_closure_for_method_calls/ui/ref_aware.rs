// run-rustfix

use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

struct DerefMutExample<T> {
    value: T,
}

impl<T> Deref for DerefMutExample<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for DerefMutExample<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

fn main() {
    let _ = std::fs::read_dir(Path::new("."))
        .ok()
        .and_then(|mut entries| entries.next())
        .unwrap()
        .unwrap();
    let _ = Some(String::from("a")).map(|s| s.is_empty());
    let _ = Some(String::from("a")).map(|s| s.to_uppercase());
    let _ = Some(DerefMutExample { value: 'a' }).map(|mut x| x.make_ascii_uppercase());

    // negative test: `Iterator`
    let _ = [String::from("a")].into_iter().map(|s| s.is_empty());

    // negative test: `Result`
    let _ = Path::new(".")
        .metadata()
        .and_then(|metadata| metadata.modified());
}
