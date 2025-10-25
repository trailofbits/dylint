#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_lexer;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_parse;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;

use rustc_lint::LateLintPass;

dylint_linting::declare_late_lint! {
    ///  ### What it does
    ///  
    ///  It enforces a certain relative order among functions defined within a module.
    ///  
    ///  ### Why is this bad?
    ///  
    ///  Without a certain order it's really bad to navigate through the modules.
    ///  
    ///  ### Example
    ///  
    ///  ```rust
    ///  fn bar() { ... }
    ///  
    ///  fn foo() {
    ///      bar();
    ///  }
    ///  ```
    ///  
    ///  Use instead:
    ///  
    ///  ```rust
    ///  fn foo() {
    ///      bar();
    ///  }
    ///  
    ///  fn bar() { ... }
    pub NON_TOPOLOGICALLY_SORTED_FUNCTIONS,
    Warn,
    "Enforce callers before callees and consistent order of callees (module-local functions)"
}

impl<'tcx> LateLintPass<'tcx> for NonTopologicallySortedFunctions {
    // A list of things you might check can be found here:
    // https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
