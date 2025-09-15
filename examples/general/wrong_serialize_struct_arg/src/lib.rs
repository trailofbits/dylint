#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{
    consts::{ConstEvalCtxt, Constant},
    diagnostics::span_lint_and_then,
};
use dylint_internal::{match_def_path, paths};
use rustc_hir::{Block, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for Serde serialization method calls whose `len` argument does not match the number of
    /// subsequent `serialize_field` or `serialize_element` calls. This includes:
    ///
    /// - `serialize_struct` (expects `serialize_field`)
    /// - `serialize_struct_variant` (expects `serialize_field`)
    /// - `serialize_tuple_struct` (expects `serialize_field`)
    /// - `serialize_tuple_variant` (expects `serialize_field`)
    /// - `serialize_tuple` (expects `serialize_element`)
    ///
    /// ### Why is this bad?
    ///
    /// The [`serde` documentation] is unclear on whether the `len` argument is meant to be a hint.
    /// Even if it is just a hint, there's no telling what real-world implementations will do with
    /// that argument. Thus, ensuring that the argument is correct helps protect against
    /// implementations that expect it to be correct, even if such implementations are only hypothetical.
    ///
    /// ### Examples
    ///
    /// ```rust
    /// # use serde::ser::{Serialize, SerializeStruct, Serializer, SerializeTuple};
    /// # struct Color { r: u8, g: u8, b: u8 }
    /// # impl Serialize for Color {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut state = serializer.serialize_struct("Color", 1)?; // `len` is 1, but 3 fields follow
    /// state.serialize_field("r", &self.r)?;
    /// state.serialize_field("g", &self.g)?;
    /// state.serialize_field("b", &self.b)?;
    /// state.end()
    /// #     }
    /// # }
    ///
    /// # struct MyPair(u8, u8); // Newtype wrapper, like a tuple struct
    /// # impl Serialize for MyPair {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut tup = serializer.serialize_tuple(1)?; // `len` is 1, but 2 elements follow
    /// tup.serialize_element(&self.0)?;
    /// tup.serialize_element(&self.1)?;
    /// tup.end()
    /// #     }
    /// # }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use serde::ser::{Serialize, SerializeStruct, Serializer, SerializeTuple};
    /// # struct Color { r: u8, g: u8, b: u8 }
    /// # impl Serialize for Color {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut state = serializer.serialize_struct("Color", 3)?;
    /// state.serialize_field("r", &self.r)?;
    /// state.serialize_field("g", &self.g)?;
    /// state.serialize_field("b", &self.b)?;
    /// state.end()
    /// #     }
    /// # }
    ///
    /// # struct MyPair(u8, u8); // Newtype wrapper, like a tuple struct
    /// # impl Serialize for MyPair {
    /// #     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// let mut tup = serializer.serialize_tuple(2)?;
    /// tup.serialize_element(&self.0)?;
    /// tup.serialize_element(&self.1)?;
    /// tup.end()
    /// #     }
    /// # }
    /// ```
    ///
    /// The same principle applies to other serialization methods like `serialize_struct_variant`,
    /// `serialize_tuple_struct`, and `serialize_tuple_variant`.
    ///
    /// [`serde` documentation]: https://docs.rs/serde/latest/serde/trait.Serializer.html#tymethod.serialize_struct
    pub WRONG_SERIALIZE_STRUCT_ARG,
    Warn,
    "calls to serialization methods with incorrect `len` arguments",
    WrongSerializeStructArg::default()
}

struct SerializationState {
    parent_serialize_span: Span,
    len: u128,
    child_call_spans: Vec<Span>,
    kind: SerializeKind,
}

#[derive(Debug, PartialEq, Eq)]
enum SerializeKind {
    Struct,
    StructVariant,
    TupleStruct,
    TupleVariant,
    Tuple,
}

#[derive(Default)]
struct WrongSerializeStructArg {
    /// `stack` contains one vector for each nested block. The inner vector contains one element
    /// for each serialization call within the block.
    stack: Vec<Vec<SerializationState>>,
}

impl<'tcx> LateLintPass<'tcx> for WrongSerializeStructArg {
    fn check_block(&mut self, _: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        self.stack.push(Vec::new());
    }

    fn check_block_post(&mut self, cx: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        let vec = self.stack.pop().unwrap();

        for SerializationState {
            parent_serialize_span,
            len,
            child_call_spans,
            kind,
        } in vec
        {
            let n = child_call_spans.len();

            if len == n as u128 {
                continue;
            }

            let (method_name, child_method_name) = match kind {
                SerializeKind::Struct => ("serialize_struct", "serialize_field"),
                SerializeKind::StructVariant => ("serialize_struct_variant", "serialize_field"),
                SerializeKind::TupleStruct => ("serialize_tuple_struct", "serialize_field"),
                SerializeKind::TupleVariant => ("serialize_tuple_variant", "serialize_field"),
                SerializeKind::Tuple => ("serialize_tuple", "serialize_element"),
            };

            span_lint_and_then(
                cx,
                WRONG_SERIALIZE_STRUCT_ARG,
                parent_serialize_span,
                format!(
                    "`{method_name}` call's `len` argument is {len}, but number of \
                     `{child_method_name}` calls is {n}"
                ),
                |diag| {
                    for (i, span) in child_call_spans.into_iter().enumerate() {
                        diag.span_note(
                            span,
                            format!("`{child_method_name}` call {} of {n}", i + 1),
                        );
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

        // Check for all serialization method types
        if let Some((kind, len)) =
            if match_def_path(cx, method_def_id, &paths::SERDE_CORE_SERIALIZE_STRUCT)
                && let [_, arg] = args
                && let Some(Constant::Int(len)) = ConstEvalCtxt::new(cx).eval(arg)
            {
                Some((SerializeKind::Struct, len))
            } else if match_def_path(
                cx,
                method_def_id,
                &paths::SERDE_CORE_SERIALIZE_STRUCT_VARIANT,
            ) && let [_, _, _, arg] = args
                && let Some(Constant::Int(len)) = ConstEvalCtxt::new(cx).eval(arg)
            {
                Some((SerializeKind::StructVariant, len))
            } else if match_def_path(cx, method_def_id, &paths::SERDE_CORE_SERIALIZE_TUPLE_STRUCT)
                && let [_, arg] = args
                && let Some(Constant::Int(len)) = ConstEvalCtxt::new(cx).eval(arg)
            {
                Some((SerializeKind::TupleStruct, len))
            } else if match_def_path(
                cx,
                method_def_id,
                &paths::SERDE_CORE_SERIALIZE_TUPLE_VARIANT,
            ) && let [_, _, _, arg] = args
                && let Some(Constant::Int(len)) = ConstEvalCtxt::new(cx).eval(arg)
            {
                Some((SerializeKind::TupleVariant, len))
            } else if match_def_path(cx, method_def_id, &paths::SERDE_CORE_SERIALIZE_TUPLE)
            && let [arg] = args // serialize_tuple(len) -> only one argument after self
            && let Some(Constant::Int(len)) = ConstEvalCtxt::new(cx).eval(arg)
            {
                Some((SerializeKind::Tuple, len))
            } else {
                None
            }
        {
            self.stack.last_mut().unwrap().push(SerializationState {
                parent_serialize_span: expr.span,
                len,
                child_call_spans: Vec::new(),
                kind,
            });
            return;
        }

        // Check for serialize_field or serialize_element calls based on the active serialization
        // kind
        if let Some(last_block_states) = self.stack.last_mut()
            && let Some(active_serialization) = last_block_states.last_mut()
        {
            let is_expected_child_call = match active_serialization.kind {
                SerializeKind::Struct => {
                    match_def_path(cx, method_def_id, &paths::SERDE_CORE_SERIALIZE_FIELD_STRUCT)
                }
                SerializeKind::StructVariant => match_def_path(
                    cx,
                    method_def_id,
                    &paths::SERDE_CORE_SERIALIZE_FIELD_STRUCT_VARIANT,
                ),
                SerializeKind::TupleStruct => match_def_path(
                    cx,
                    method_def_id,
                    &paths::SERDE_CORE_SERIALIZE_FIELD_TUPLE_STRUCT,
                ),
                SerializeKind::TupleVariant => match_def_path(
                    cx,
                    method_def_id,
                    &paths::SERDE_CORE_SERIALIZE_FIELD_TUPLE_VARIANT,
                ),
                SerializeKind::Tuple => {
                    match_def_path(cx, method_def_id, &paths::SERDE_CORE_SERIALIZE_ELEMENT)
                }
            };
            if is_expected_child_call {
                active_serialization.child_call_spans.push(expr.span);
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
