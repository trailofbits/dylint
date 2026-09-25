use super::variables::IsVariable;
use crate::{Binding, Error, ErrorKind, Visitable};
use std::any::{Any, type_name};
use syn::spanned::Spanned;

trait SynNode: Any + Spanned {}

struct NodeEntryExit<'ast> {
    enter: bool,
    type_name: &'static str,
    node: &'ast dyn SynNode,
    span: rustc_span::Span,
}

impl std::fmt::Debug for NodeEntryExit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:?}: {}({}) ({:p})",
            self.span,
            if self.enter { "Enter" } else { "Exit" },
            self.type_name,
            std::ptr::from_ref(self.node).cast::<()>(),
        ))
    }
}

pub trait Unify {
    /// Arguments
    ///
    /// - `self` is the "scrutinee", i.e., a concrete `syn` AST with no "holes".
    /// - `hir_span` is used to produce error messages.
    /// - `pattern` is a `syn` AST with "holes".
    /// - `bindings` records the subtrees that fill the holes.
    fn unify(
        &self,
        span: rustc_span::Span,
        pattern: &Self,
        bindings: &mut Vec<Binding>,
    ) -> std::result::Result<(), Error>;
}

impl<T: SynNode + Visitable> Unify for T {
    fn unify(
        &self,
        span: rustc_span::Span,
        pattern: &Self,
        bindings: &mut Vec<Binding>,
    ) -> std::result::Result<(), Error> {
        let node_entry_exits = produce(span, self);
        consume(&mut node_entry_exits.as_slice(), pattern, bindings)
    }
}

fn produce(
    root_hir_span: rustc_span::Span,
    scrutinee: &(impl SynNode + Visitable),
) -> Vec<NodeEntryExit<'_>> {
    let root_pm2_span = scrutinee.span();
    let mut visitor = Producer {
        root_hir_span,
        root_pm2_span,
        nodes: Vec::new(),
    };
    scrutinee.visit(&mut visitor);
    visitor.nodes
}

fn consume<'unify, 'ast>(
    node_entry_exits: &'unify mut &'unify [NodeEntryExit<'ast>],
    pattern: &'unify (impl SynNode + Visitable),
    bindings: &'ast mut Vec<Binding>,
) -> std::result::Result<(), Error>
where
    'unify: 'ast,
{
    let mut visitor = Consumer {
        node_entry_exits,
        pending_exits: Vec::new(),
        bindings,
        n_leaves: 0,
        error: None,
    };
    pattern.visit(&mut visitor);
    assert!(
        visitor.error.is_some() || visitor.pending_exits.is_empty(),
        "pending exits remain: {:#?}",
        visitor.pending_exits
    );
    if let Some(error) = visitor.error {
        Err(error)
    } else {
        Ok(())
    }
}

struct Producer<'ast> {
    root_hir_span: rustc_span::Span,
    root_pm2_span: proc_macro2::Span,
    nodes: Vec<NodeEntryExit<'ast>>,
}

struct Consumer<'unify, 'ast> {
    node_entry_exits: &'unify [NodeEntryExit<'ast>],
    pending_exits: Vec<&'unify NodeEntryExit<'ast>>,
    bindings: &'ast mut Vec<Binding>,
    n_leaves: usize,
    error: Option<Error>,
}

impl<'ast> Producer<'ast> {
    fn visit_inner<T: SynNode + Visitable>(&mut self, node: &'ast T) {
        let type_name = type_name::<T>();
        let node_pm2_span = node.span();
        let node_hir_span = shrink_hir_span(self.root_hir_span, self.root_pm2_span, node_pm2_span);
        self.nodes.push(NodeEntryExit {
            enter: true,
            type_name,
            node,
            span: node_hir_span,
        });
        T::visit_children(node, self);
        self.nodes.push(NodeEntryExit {
            enter: false,
            type_name,
            node,
            span: node_hir_span,
        });
    }
}

impl<'ast> Consumer<'_, 'ast> {
    fn visit_inner<T: IsVariable + PartialEq + Spanned + Visitable + std::fmt::Debug + 'static>(
        &mut self,
        pattern: &'ast T,
    ) {
        // smoelius: `error` could have been set in a call to `visit_inner` in a previous sibling.
        if self.error.is_some() {
            return;
        }

        let [
            NodeEntryExit {
                enter: true,
                type_name: _,
                node: scrutinee,
                span,
            },
            ..,
        ] = self.node_entry_exits
        else {
            // smoelius: The scrutinee and pattern could both be of the same type (e.g.,
            // `syn::Expr`), but be different variants of that type (e.g., `syn::Expr::Block` and
            // `syn::Expr::Call`), which can trigger this `else` case. Hence, this `else` case
            // should not cause a panic.
            self.set_error(Error::new(rustc_span::DUMMY_SP, ErrorKind::NoMatch));
            return;
        };
        self.pending_exits.push(&self.node_entry_exits[0]);
        pop_front(&mut self.node_entry_exits, 1);

        if pattern.is_variable() {
            self.bindings.push(Binding::new::<T>(*span));
            // smoelius: If the next `position` fails, the `assert` just after this `if`-`else` will
            // (intentionally) panic.
            if let Some(n) = self
                .node_entry_exits
                .iter()
                .position(is_node_exit_for(*scrutinee))
            {
                pop_front(&mut self.node_entry_exits, n);
            }
        } else {
            let n_bindings_before = self.bindings.len();
            let n_leaves_before = self.n_leaves;
            T::visit_children(pattern, self);
            if self.error.is_some() {
                return;
            }
            let n_bindings_after = self.bindings.len();
            let n_leaves_after = self.n_leaves;

            // smoelius: If the call to `T::visit_children` created no new bindings and did not
            // change `self.n_leaves`, then `pattern` is a leaf and must match `scrutinee` exactly.
            if n_bindings_before == n_bindings_after && n_leaves_before == n_leaves_after {
                self.n_leaves += 1;
                if (*scrutinee as &dyn Any).downcast_ref::<T>() != Some(pattern) {
                    self.set_error(Error::new(*span, ErrorKind::NoMatch));
                    return;
                }
            }
        }

        // smoelius: There is no node for `syn`'s `Punctuated`. So when `scrutinee`'s children were
        // visited, new node entries could have been uncovered. Thus, we cannot assume that the next
        // element of `self.node_entry_exits` is an exit for `scrutinee`. If the next element of
        // `self.node_entry_exits` is an entry for any node, the match has failed.
        if let [
            NodeEntryExit {
                enter: true,
                type_name: _,
                node: _,
                span: span_sibling,
            },
            ..,
        ] = self.node_entry_exits
        {
            self.set_error(Error::new(*span_sibling, ErrorKind::NoMatch));
            return;
        }

        // smoelius: If the next element is not an entry, then it should be an exit specifically for
        // `scrutinee`.
        assert!(
            matches!(self.node_entry_exits, [node_entry_exit, ..] if is_node_exit_for(*scrutinee)(node_entry_exit)),
            "failed to find node exit: {:#?} {:#?}",
            self.pending_exits,
            self.node_entry_exits,
        );
        pop_front(&mut self.node_entry_exits, 1);
        self.pending_exits.pop().unwrap();
    }

    fn set_error(&mut self, error: Error) {
        assert!(
            self.error.is_none(),
            "error was already set: {:?}",
            self.error
        );
        self.error = Some(error);
    }
}

// smoelius: The `proc_macro2::Span`s that `match_hir` constructs refer to text, not source files.
// Hence, their lines and columns are relative.
pub(crate) fn shrink_hir_span(
    hir_span: rustc_span::Span,
    pm2_span_larger: proc_macro2::Span,
    pm2_span_smaller: proc_macro2::Span,
) -> rustc_span::Span {
    let byte_range_larger = pm2_span_larger.byte_range();
    let byte_range_smaller = pm2_span_smaller.byte_range();

    assert!(byte_range_larger.start <= byte_range_smaller.start);
    assert!(byte_range_smaller.end <= byte_range_larger.end);

    let start_trim = u32::try_from(byte_range_smaller.start - byte_range_larger.start).unwrap();
    let end_trim = u32::try_from(byte_range_larger.end - byte_range_smaller.end).unwrap();

    hir_span
        .with_lo(hir_span.lo() + rustc_span::BytePos(start_trim))
        .with_hi(hir_span.hi() - rustc_span::BytePos(end_trim))
}

fn pop_front<T>(queue: &mut &[T], n: usize) {
    *queue = &queue[n..];
}

fn is_node_exit_for<'ast>(node: &'ast dyn SynNode) -> impl Fn(&NodeEntryExit<'ast>) -> bool {
    |node_entry_exit| {
        if let NodeEntryExit {
            enter: false,
            type_name: _,
            node: other,
            span: _,
        } = node_entry_exit
        {
            std::ptr::from_ref::<dyn SynNode>(node) == *other
        } else {
            false
        }
    }
}

macro_rules! impl_is_variable {
    ($ty:ident) => {
        impl IsVariable for syn::$ty {}
    };
}

macro_rules! impl_syn_node {
    ($ty:ident) => {
        impl SynNode for syn::$ty {}
    };
}

include!(concat!(env!("OUT_DIR"), "/unify_variable.rs"));
include!(concat!(env!("OUT_DIR"), "/unify_node.rs"));
include!(concat!(env!("OUT_DIR"), "/unify_producer.rs"));
include!(concat!(env!("OUT_DIR"), "/unify_consumer.rs"));
