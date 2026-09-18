The file hir.rs, in the same directory as this README, is from the following location:

https://github.com/rust-lang/rust/blob/f0308938ba39bc3377f22f7479654ba32e9c233f/compiler/rustc_hir/src/hir.rs

The file is used by build.rs to implement the `TypeNameGetter` trait, which the span-to-hir-id map
uses.

The file visit.rs, also in the same directory as this README, is from the following location:

https://github.com/dtolnay/syn/blob/6eb82a997589cf9c5b2bb36716443a19c4440c5e/src/gen/visit.rs

The file is used by build.rs to implement the `Unify` and `Visitable` traits.
