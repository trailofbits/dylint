use crate::common::create;
use inflections::Inflect;
use quote::ToTokens;
use std::{fs::read_to_string, io::Write};
use syn::{FnArg, Item, PatType, TraitItem, Type, TypePath, TypeReference, parse_file};

// smoelius: `ToTokens` implementations do not exist for these `syn` types, even though
// `Visit::visit_...` methods do.
const NO_TO_TOKENS: &[&str] = &[
    "AttrStyle",
    "Data",
    "DataEnum",
    "DataStruct",
    "DataUnion",
    "FieldMutability",
    "ImplRestriction",
    "LocalInit",
    "MacroDelimiter",
    "Span",
    "QSelf",
];

// smoelius: Manual `IsVariable` implementations exist for these `syn` types.
const MANUAL_IS_VARIABLE: &[&str] = &[
    "Expr",
    "GenericArgument",
    "Ident",
    "Member",
    "Pat",
    "Path",
    "PathSegment",
    "Stmt",
    "Type",
];

pub fn emit_impls(out_dir: &str) {
    let contents = read_to_string("assets/visit.rs").unwrap();
    let syn_file =
        parse_file(&contents).unwrap_or_else(|_| panic!("Failed to parse: {contents:?}"));

    let visit_trait = syn_file
        .items
        .iter()
        .find_map(|item| {
            if let Item::Trait(item_trait) = item
                && item_trait.ident == "Visit"
            {
                Some(item_trait)
            } else {
                None
            }
        })
        .unwrap();

    let mut unify_variable = create(out_dir, "unify_variable.rs").unwrap();
    let mut unify_node = create(out_dir, "unify_node.rs").unwrap();
    let mut unify_producer = create(out_dir, "unify_producer.rs").unwrap();
    let mut unify_consumer = create(out_dir, "unify_consumer.rs").unwrap();
    let mut visitable = create(out_dir, "visitable.rs").unwrap();

    writeln!(
        unify_producer,
        "impl<'ast> syn::visit::Visit<'ast> for Producer<'ast> {{"
    )
    .unwrap();
    writeln!(
        unify_consumer,
        "impl<'ast> syn::visit::Visit<'ast> for Consumer<'_, 'ast> {{"
    )
    .unwrap();

    for trait_item in &visit_trait.items {
        let Some(ty) = is_visit_fn(trait_item) else {
            continue;
        };

        if NO_TO_TOKENS.contains(&ty.as_str()) {
            continue;
        }

        let ty_snake = ty.to_snake_case();

        if !MANUAL_IS_VARIABLE.contains(&ty.as_str()) {
            writeln!(unify_variable, "impl_is_variable!({ty});").unwrap();
        }

        writeln!(unify_node, "impl_syn_node!({ty});").unwrap();

        for file in [&mut unify_producer, &mut unify_consumer] {
            writeln!(
                file,
                "\
    fn visit_{ty_snake}(&mut self, node: &'ast syn::{ty}) {{
        self.visit_inner(node);
    }}"
            )
            .unwrap();
        }

        writeln!(visitable, "impl_visitable!({ty}, {ty_snake});").unwrap();
    }

    writeln!(unify_producer, "}}").unwrap();
    writeln!(unify_consumer, "}}").unwrap();
}

fn is_visit_fn(trait_item: &TraitItem) -> Option<String> {
    if let TraitItem::Fn(trait_item_fn) = trait_item
        && let [_, FnArg::Typed(PatType { ty, .. })] = trait_item_fn
            .sig
            .inputs
            .iter()
            .collect::<Vec<_>>()
            .as_slice()
        && let Type::Reference(TypeReference { elem, .. }) = &**ty
        && let Type::Path(TypePath { qself: None, path }) = &**elem
        && let Some(segment) = path.segments.last()
    {
        Some(segment.to_token_stream().to_string())
    } else {
        None
    }
}
