use crate::common::create;
use std::{fs::read_to_string, io::Write};
use syn::{
    Fields, FieldsUnnamed, Item, Path as SynPath, Type, TypePath, TypeReference, parse_file,
};

pub fn emit_impls(out_dir: &str) {
    let contents = read_to_string("assets/hir.rs").unwrap();
    let syn_file =
        parse_file(&contents).unwrap_or_else(|_| panic!("Failed to parse: {contents:?}"));

    let node_enum = syn_file
        .items
        .iter()
        .find_map(|item| {
            if let Item::Enum(item_enum) = item
                && item_enum.ident == "Node"
            {
                Some(item_enum)
            } else {
                None
            }
        })
        .unwrap();

    let mut type_name = create(out_dir, "type_name.rs").unwrap();

    writeln!(
        type_name,
        "pub fn type_name(node: hir::Node) -> std::option::Option<&'static str> {{
    #[allow(clippy::match_same_arms)]
    match node {{"
    )
    .unwrap();

    for variant in &node_enum.variants {
        if let Fields::Unnamed(FieldsUnnamed { unnamed, .. }) = &variant.fields
            && unnamed.len() <= 1
            && let Some(field) = unnamed.first()
            && let ty = peel_refs(&field.ty)
            && let Type::Path(TypePath {
                qself: None,
                path:
                    SynPath {
                        leading_colon: None,
                        segments,
                        ..
                    },
            }) = ty
            && segments.len() <= 1
            && let Some(segment) = segments.first()
        {
            // smoelius: `Node::Err` must be handled specially.
            let expr = if segment.ident == "Span" {
                String::from("None")
            } else {
                format!("TypeNameGetter::<hir::{}>::type_name()", segment.ident)
            };
            writeln!(
                type_name,
                "        hir::Node::{}(_) => {expr},",
                variant.ident
            )
            .unwrap();
        }
        // smoelius: `Node::Synthetic` must be handled specially.
        else if matches!(&variant.fields, Fields::Unit) {
            writeln!(type_name, "        hir::Node::{} => None,", variant.ident,).unwrap();
        } else {
            panic!("failed to emit match arm for variant `{}`", variant.ident);
        }
    }

    writeln!(
        type_name,
        "    }}
}}"
    )
    .unwrap();
}

fn peel_refs(mut ty: &Type) -> &Type {
    while let Type::Reference(TypeReference { elem, .. }) = ty {
        ty = elem;
    }
    ty
}
