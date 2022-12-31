#![allow(dead_code)]
#![warn(clippy::module_name_repetitions, clippy::unused_self)]

fn main() {}

#[allow(clippy::module_name_repetitions)]
mod item {
    pub struct ItemStruct;
}

mod impl_item {
    struct S;

    #[allow(clippy::unused_self)]
    impl S {
        fn foo(&self) {}
    }
}

#[allow(clippy::unwrap_used)]
fn stmt() {
    Some(()).unwrap();
}

#[allow(clippy::unwrap_used)]
mod nested {
    fn stmt() {
        Some(()).unwrap();
    }
}

#[allow(clippy::module_name_repetitions, clippy::unwrap_used)]
fn multiple_allows() {
    Some(()).unwrap();
}

#[cfg_attr(all(), allow(clippy::unwrap_used))]
fn cfg_attr() {
    Some(()).unwrap();
}

mod negative_item {
    #[allow(clippy::module_name_repetitions)]
    pub struct NegativeItemStruct;
}

mod negative_impl_item {
    struct S;

    impl S {
        #[allow(clippy::unused_self)]
        fn foo(&self) {}
    }
}

fn negative_stmt() {
    #[allow(clippy::unwrap_used)]
    Some(()).unwrap();
}

#[allow(clippy::unwrap_used)]
fn negative_multiple_diagnostics() {
    Some(()).unwrap();
    Some(()).unwrap();
}
