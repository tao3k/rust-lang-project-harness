use quote::ToTokens;

use super::facts::path_segments_text;

pub(super) fn primitive_contract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => primitive_contract_path_name(type_path),
        syn::Type::Reference(reference) => {
            primitive_contract_type_name(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => None,
    }
}

pub(super) fn flag_contract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => flag_contract_path_name(type_path),
        syn::Type::Reference(reference) => {
            flag_contract_type_name(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => None,
    }
}

pub(super) fn tuple_api_contract_types(ty: &syn::Type) -> Option<Vec<String>> {
    match ty {
        syn::Type::Tuple(tuple) => semantic_tuple_contract_types(&tuple.elems),
        syn::Type::Paren(paren) => tuple_api_contract_types(&paren.elem),
        syn::Type::Group(group) => tuple_api_contract_types(&group.elem),
        syn::Type::Reference(reference) => tuple_api_contract_types(&reference.elem),
        syn::Type::Path(type_path) => wrapper_tuple_contract_types(type_path),
        _ => None,
    }
}

pub(super) fn dynamic_json_api_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Paren(paren) => dynamic_json_api_type_name(&paren.elem),
        syn::Type::Group(group) => dynamic_json_api_type_name(&group.elem),
        syn::Type::Reference(reference) => dynamic_json_api_type_name(&reference.elem),
        syn::Type::Path(type_path) => dynamic_json_path_type_name(type_path),
        _ => None,
    }
}

fn dynamic_json_path_type_name(type_path: &syn::TypePath) -> Option<String> {
    if type_path.qself.is_some() {
        return None;
    }
    let path_text = path_segments_text(&type_path.path);
    if path_text == "Value" || path_text == "serde_json::Value" {
        return Some(type_path.to_token_stream().to_string());
    }
    type_path.path.segments.iter().find_map(|segment| {
        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return None;
        };
        args.args.iter().find_map(|argument| {
            let syn::GenericArgument::Type(argument_type) = argument else {
                return None;
            };
            dynamic_json_api_type_name(argument_type)
        })
    })
}

fn wrapper_tuple_contract_types(type_path: &syn::TypePath) -> Option<Vec<String>> {
    let terminal = type_path.path.segments.last()?;
    let terminal_name = terminal.ident.to_string();
    if terminal_name != "Option" && terminal_name != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &terminal.arguments else {
        return None;
    };
    let mut generic_types = args.args.iter().filter_map(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return None;
        };
        Some(ty)
    });
    let value_type = generic_types.next()?;
    tuple_api_contract_types(value_type)
}

fn semantic_tuple_contract_types(
    elems: &syn::punctuated::Punctuated<syn::Type, syn::token::Comma>,
) -> Option<Vec<String>> {
    if elems.len() < 2 {
        return None;
    }
    let element_contract_types = elems
        .iter()
        .filter_map(|elem| {
            primitive_contract_type_name(elem).or_else(|| flag_contract_type_name(elem))
        })
        .collect::<Vec<_>>();
    (element_contract_types.len() >= 2).then_some(element_contract_types)
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
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
    )
}
