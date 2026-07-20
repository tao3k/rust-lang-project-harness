//! Native Rust public data-shape facts.

use quote::ToTokens;
use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicStructFieldSyntax {
    pub line: usize,
    pub struct_line: usize,
    pub struct_name: String,
    pub field_name: String,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicEnumVariantFieldSyntax {
    pub line: usize,
    pub enum_line: usize,
    pub variant_line: usize,
    pub enum_name: String,
    pub variant_name: String,
    pub field_name: String,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicEnumTupleVariantFieldSyntax {
    pub line: usize,
    pub enum_line: usize,
    pub variant_line: usize,
    pub enum_name: String,
    pub variant_name: String,
    pub field_index: usize,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicTypeGenericBoundSyntax {
    pub line: usize,
    pub type_line: usize,
    pub type_kind: &'static str,
    pub type_name: String,
    pub param_name: String,
    pub bound_name: String,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicTypeAliasSyntax {
    pub line: usize,
    pub alias_name: String,
    pub target_type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

pub(crate) fn public_struct_field_syntax(item: &syn::Item) -> Vec<RustPublicStructFieldSyntax> {
    let syn::Item::Struct(item_struct) = item else {
        return Vec::new();
    };
    if !is_public_visibility(&item_struct.vis) {
        return Vec::new();
    }
    let syn::Fields::Named(fields) = &item_struct.fields else {
        return Vec::new();
    };
    let struct_name = item_struct.ident.to_string();
    let struct_line = item_struct.ident.span().start().line.max(1);
    let is_test_context = attrs_have_cfg_test(&item_struct.attrs);
    fields
        .named
        .iter()
        .filter_map(|field| {
            if !is_public_visibility(&field.vis) {
                return None;
            }
            let ident = field.ident.as_ref()?;
            Some(RustPublicStructFieldSyntax {
                line: ident.span().start().line.max(1),
                struct_line,
                struct_name: struct_name.clone(),
                field_name: ident.to_string(),
                type_text: field.ty.to_token_stream().to_string(),
                primitive_contract_type: primitive_contract_type_name(&field.ty),
                flag_contract_type: flag_contract_type_name(&field.ty),
                is_test_context,
            })
        })
        .collect()
}

pub(crate) fn public_type_alias_syntax(item: &syn::Item) -> Vec<RustPublicTypeAliasSyntax> {
    let syn::Item::Type(item_type) = item else {
        return Vec::new();
    };
    if !is_public_visibility(&item_type.vis) {
        return Vec::new();
    }
    vec![RustPublicTypeAliasSyntax {
        line: item_type.ident.span().start().line.max(1),
        alias_name: item_type.ident.to_string(),
        target_type_text: item_type.ty.to_token_stream().to_string(),
        primitive_contract_type: primitive_contract_type_name(&item_type.ty),
        flag_contract_type: flag_contract_type_name(&item_type.ty),
        is_test_context: attrs_have_cfg_test(&item_type.attrs),
    }]
}

pub(crate) fn public_type_generic_bound_syntax(
    item: &syn::Item,
) -> Vec<RustPublicTypeGenericBoundSyntax> {
    match item {
        syn::Item::Struct(item_struct) if is_public_visibility(&item_struct.vis) => {
            generic_bound_syntax(
                "struct",
                item_struct.ident.to_string(),
                item_struct.ident.span().start().line.max(1),
                &item_struct.generics,
                attrs_have_cfg_test(&item_struct.attrs),
            )
        }
        syn::Item::Enum(item_enum) if is_public_visibility(&item_enum.vis) => generic_bound_syntax(
            "enum",
            item_enum.ident.to_string(),
            item_enum.ident.span().start().line.max(1),
            &item_enum.generics,
            attrs_have_cfg_test(&item_enum.attrs),
        ),
        _ => Vec::new(),
    }
}

pub(crate) fn public_enum_variant_field_syntax(
    item: &syn::Item,
) -> Vec<RustPublicEnumVariantFieldSyntax> {
    let syn::Item::Enum(item_enum) = item else {
        return Vec::new();
    };
    if !is_public_visibility(&item_enum.vis) {
        return Vec::new();
    }
    let enum_name = item_enum.ident.to_string();
    let enum_line = item_enum.ident.span().start().line.max(1);
    let enum_test_context = attrs_have_cfg_test(&item_enum.attrs);
    item_enum
        .variants
        .iter()
        .flat_map(|variant| {
            let syn::Fields::Named(fields) = &variant.fields else {
                return Vec::new();
            };
            let variant_name = variant.ident.to_string();
            let variant_line = variant.ident.span().start().line.max(1);
            let is_test_context = enum_test_context || attrs_have_cfg_test(&variant.attrs);
            fields
                .named
                .iter()
                .filter_map(|field| {
                    let ident = field.ident.as_ref()?;
                    Some(RustPublicEnumVariantFieldSyntax {
                        line: ident.span().start().line.max(1),
                        enum_line,
                        variant_line,
                        enum_name: enum_name.clone(),
                        variant_name: variant_name.clone(),
                        field_name: ident.to_string(),
                        type_text: field.ty.to_token_stream().to_string(),
                        primitive_contract_type: primitive_contract_type_name(&field.ty),
                        flag_contract_type: flag_contract_type_name(&field.ty),
                        is_test_context,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn public_enum_tuple_variant_field_syntax(
    item: &syn::Item,
) -> Vec<RustPublicEnumTupleVariantFieldSyntax> {
    let syn::Item::Enum(item_enum) = item else {
        return Vec::new();
    };
    if !is_public_visibility(&item_enum.vis) {
        return Vec::new();
    }
    let enum_name = item_enum.ident.to_string();
    let enum_line = item_enum.ident.span().start().line.max(1);
    let enum_test_context = attrs_have_cfg_test(&item_enum.attrs);
    item_enum
        .variants
        .iter()
        .flat_map(|variant| {
            let syn::Fields::Unnamed(fields) = &variant.fields else {
                return Vec::new();
            };
            let variant_name = variant.ident.to_string();
            let variant_line = variant.ident.span().start().line.max(1);
            let is_test_context = enum_test_context || attrs_have_cfg_test(&variant.attrs);
            fields
                .unnamed
                .iter()
                .enumerate()
                .map(
                    |(field_index, field)| RustPublicEnumTupleVariantFieldSyntax {
                        line: field.ty.span().start().line.max(1),
                        enum_line,
                        variant_line,
                        enum_name: enum_name.clone(),
                        variant_name: variant_name.clone(),
                        field_index,
                        type_text: field.ty.to_token_stream().to_string(),
                        primitive_contract_type: primitive_contract_type_name(&field.ty),
                        flag_contract_type: flag_contract_type_name(&field.ty),
                        is_test_context,
                    },
                )
                .collect::<Vec<_>>()
        })
        .collect()
}

fn generic_bound_syntax(
    type_kind: &'static str,
    type_name: String,
    type_line: usize,
    generics: &syn::Generics,
    is_test_context: bool,
) -> Vec<RustPublicTypeGenericBoundSyntax> {
    let mut bounds = Vec::new();
    for generic_param in &generics.params {
        let syn::GenericParam::Type(type_param) = generic_param else {
            continue;
        };
        bounds.extend(type_param.bounds.iter().filter_map(|bound| {
            notable_data_bound_name(bound).map(|bound_name| RustPublicTypeGenericBoundSyntax {
                line: type_param.ident.span().start().line.max(1),
                type_line,
                type_kind,
                type_name: type_name.clone(),
                param_name: type_param.ident.to_string(),
                bound_name,
                is_test_context,
            })
        }));
    }
    if let Some(where_clause) = &generics.where_clause {
        for predicate in &where_clause.predicates {
            let syn::WherePredicate::Type(type_predicate) = predicate else {
                continue;
            };
            let param_name = type_predicate.bounded_ty.to_token_stream().to_string();
            bounds.extend(type_predicate.bounds.iter().filter_map(|bound| {
                notable_data_bound_name(bound).map(|bound_name| RustPublicTypeGenericBoundSyntax {
                    line: type_predicate.bounded_ty.span().start().line.max(1),
                    type_line,
                    type_kind,
                    type_name: type_name.clone(),
                    param_name: param_name.clone(),
                    bound_name,
                    is_test_context,
                })
            }));
        }
    }
    bounds
}

fn notable_data_bound_name(bound: &syn::TypeParamBound) -> Option<String> {
    let syn::TypeParamBound::Trait(trait_bound) = bound else {
        return None;
    };
    let terminal = trait_bound.path.segments.last()?.ident.to_string();
    is_data_structure_bound_hazard(&terminal).then_some(terminal)
}

fn is_data_structure_bound_hazard(bound_name: &str) -> bool {
    matches!(
        bound_name,
        "Clone"
            | "PartialEq"
            | "PartialOrd"
            | "Debug"
            | "Display"
            | "Default"
            | "Error"
            | "Serialize"
            | "Deserialize"
            | "DeserializeOwned"
    )
}

fn primitive_contract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => primitive_contract_path_name(type_path),
        syn::Type::Reference(reference) => {
            primitive_contract_type_name(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => None,
    }
}

fn flag_contract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => flag_contract_path_name(type_path),
        syn::Type::Reference(reference) => {
            flag_contract_type_name(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => None,
    }
}

fn primitive_contract_path_name(type_path: &syn::TypePath) -> Option<String> {
    let terminal = type_path.path.segments.last()?;
    let terminal_name = terminal.ident.to_string();
    if is_string_or_integer_primitive(&terminal_name) {
        return Some(terminal_name);
    }
    let syn::PathArguments::AngleBracketed(args) = &terminal.arguments else {
        return None;
    };
    if terminal_name != "Option" {
        return None;
    }
    let mut generic_types = args.args.iter().filter_map(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return None;
        };
        primitive_contract_type_name(ty)
    });
    let inner = generic_types.next()?;
    generic_types
        .next()
        .is_none()
        .then_some(format!("{terminal_name}<{inner}>"))
}

fn flag_contract_path_name(type_path: &syn::TypePath) -> Option<String> {
    let terminal = type_path.path.segments.last()?;
    let terminal_name = terminal.ident.to_string();
    if terminal_name == "bool" {
        return Some(terminal_name);
    }
    let syn::PathArguments::AngleBracketed(args) = &terminal.arguments else {
        return None;
    };
    if terminal_name != "Option" {
        return None;
    }
    let mut generic_types = args.args.iter().filter_map(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return None;
        };
        flag_contract_type_name(ty)
    });
    let inner = generic_types.next()?;
    generic_types
        .next()
        .is_none()
        .then_some(format!("{terminal_name}<{inner}>"))
}

fn is_string_or_integer_primitive(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "str"
            | "usize"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
    )
}

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg") && attr.to_token_stream().to_string().contains("test")
}

pub(super) fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}
