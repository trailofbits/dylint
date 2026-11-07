use std::env::var;

mod common;
mod hir;
mod visit;

fn main() {
    #[cfg_attr(dylint_lib = "env_literal", allow(env_literal))]
    let out_dir = var("OUT_DIR").unwrap();

    hir::emit_impls(&out_dir);
    visit::emit_impls(&out_dir);

    println!("cargo:rerun-if-changed=assets/hir.rs");
    println!("cargo:rerun-if-changed=assets/visit.rs");
}
