#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{
    consts::{constant, Constant},
    diagnostics::span_lint_and_then,
    match_def_path,
};
use dylint_internal::paths;
use rustc_hir::{Block, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_linting::impl_late_lint! {
    /// ### What it does
    /// Checks for `serialize_struct` calls whose `len` argument does not match the number of
    /// subsequent `serialize_field` calls.
    ///
    /// ### Why is this bad?
    /// The [`serde` documentation] is unclear on whether the `len` argument is meant to be a hint.
    /// Even if it is just a hint, there's no telling what real-world implementations will do with
    /// that argument. Thus, ensuring that the argument is correct helps protect against
    /// `SerializeStruct` implementations that expect it to be correct, even if such implementations
    /// are only hypothetical.
    ///
    /// ### Example
    /// ```rust
    /// # struct Color { r: u8, g: u8, b: u8 }
    /// # use serde::ser::{Serialize, SerializeStruct, Serializer};
    /// # impl Serialize for Color {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut state = serializer.serialize_struct("Color", 1)?; // `len` is 1
    /// state.serialize_field("r", &self.r)?;
    /// state.serialize_field("g", &self.g)?;
    /// state.serialize_field("b", &self.b)?;
    /// state.end()
    /// #     }
    /// # }
    /// ```
    /// Use instead:
    /// ```rust
    /// # struct Color { r: u8, g: u8, b: u8 }
    /// # use serde::ser::{Serialize, SerializeStruct, Serializer};
    /// # impl Serialize for Color {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut state = serializer.serialize_struct("Color", 3)?; // `len` is 3
    /// state.serialize_field("r", &self.r)?;
    /// state.serialize_field("g", &self.g)?;
    /// state.serialize_field("b", &self.b)?;
    /// state.end()
    /// #     }
    /// # }
    /// ```
    ///
    /// [`serde` documentation]: https://docs.rs/serde/latest/serde/trait.Serializer.html#tymethod.serialize_struct
    pub WRONG_SERIALIZE_STRUCT_ARG,
    Warn,
    "calls to `serialize_struct` with incorrect `len` arguments",
    WrongSerializeStructArg::default()
}

struct SerializeStruct {
    serialize_struct_span: Span,
    len: u128,
    serialize_field_spans: Vec<Span>,
}

#[derive(Default)]
struct WrongSerializeStructArg {
    /// `stack` contains one vector for each nested block. The inner vector contains one element
    /// for each `serialize_struct` call within the block.
    stack: Vec<Vec<SerializeStruct>>,
}

impl<'tcx> LateLintPass<'tcx> for WrongSerializeStructArg {
    fn check_block(&mut self, _: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        self.stack.push(Vec::new());
    }

    fn check_block_post(&mut self, cx: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        let vec = self.stack.pop().unwrap();

        for SerializeStruct {
            serialize_struct_span,
            len,
            serialize_field_spans,
        } in vec
        {
            let n = serialize_field_spans.len();

            if len == n as u128 {
                continue;
            }

            span_lint_and_then(
                cx,
                WRONG_SERIALIZE_STRUCT_ARG,
                serialize_struct_span,
                format!(
                    "`serialize_struct` call's `len` argument is {len}, but number of \
                     `serialize_field` calls is {n}"
                ),
                |diag| {
                    for (i, span) in serialize_field_spans.into_iter().enumerate() {
                        diag.span_note(span, format!("`serialize_field` call {} of {n}", i + 1));
                    }
                },
            );
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(_, _, args, _) = expr.kind else {
            return;
        };

        let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
            return;
        };

        if match_def_path(cx, method_def_id, &paths::SERDE_SERIALIZE_STRUCT)
            && let [_, arg] = args
            && let Some(Constant::Int(len)) = constant(cx, cx.typeck_results(), arg)
        {
            self.stack.last_mut().unwrap().push(SerializeStruct {
                serialize_struct_span: expr.span,
                len,
                serialize_field_spans: Vec::new(),
            });
            return;
        }

        if match_def_path(cx, method_def_id, &paths::SERDE_SERIALIZE_FIELD)
            && let Some(serialize_struct) = self.stack.last_mut().unwrap().last_mut()
        {
            serialize_struct.serialize_field_spans.push(expr.span);
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
