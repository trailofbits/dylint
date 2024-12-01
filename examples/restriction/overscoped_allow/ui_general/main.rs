#![allow(dead_code)]

fn main() {}

// smoelius: This test broken around the time `clippy::module_name_repetitions` was moved to
// `restriction`: https://github.com/rust-lang/rust-clippy/pull/13541
// I haven't yet figured out how to fix it.
#[allow(clippy::module_name_repetitions)]
mod item {
    pub struct ItemStruct;
}

#[allow(clippy::wrong_self_convention)]
mod trait_item {
    trait T {
        fn into_foo(&self) {}
    }
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
fn block_expr() {
    Some(()).unwrap()
}

// smoelius: See comment above re `clippy::module_name_repetitions` moving to `restriction`.
#[allow(clippy::module_name_repetitions)]
mod nested_item {
    mod item {
        pub struct ItemStruct;
    }
}

#[allow(clippy::wrong_self_convention)]
mod nested_trait_item {
    mod trait_item {
        trait T {
            fn into_foo(&self) {}
        }
    }
}

mod nested_impl_item {
    #[allow(clippy::unused_self)]
    mod impl_item {
        struct S;

        impl S {
            fn foo(&self) {}
        }
    }
}

#[allow(clippy::unwrap_used)]
mod nested_stmt {
    fn stmt() {
        Some(()).unwrap();
    }
}

#[allow(clippy::unwrap_used)]
mod nested_block_expr {
    fn block_expr() {
        Some(()).unwrap()
    }
}

#[cfg_attr(all(), allow(clippy::unwrap_used))]
fn cfg_attr() {
    Some(()).unwrap();
}

#[allow(clippy::module_name_repetitions, clippy::unwrap_used)]
fn multiple_allows() {
    Some(()).unwrap();
}

mod negative_item {
    #[allow(clippy::module_name_repetitions)]
    pub struct NegativeItemStruct;
}

mod negative_trait_item {
    trait T {
        #[allow(clippy::unused_self)]
        fn foo(&self) {}
    }
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

fn negative_block_expr() {
    #[allow(clippy::unwrap_used)]
    Some(()).unwrap()
}

#[allow(clippy::unwrap_used)]
fn negative_semi() {
    Some(()).unwrap() as ();
}

#[allow(clippy::unwrap_used)]
fn negative_multiple_diagnostics() {
    Some(()).unwrap();
    Some(()).unwrap() as ();
}
