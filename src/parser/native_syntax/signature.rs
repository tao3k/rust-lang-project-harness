//! Native Rust public signature facts.

use quote::ToTokens;
use syn::spanned::Spanned;

use super::{attrs_have_cfg_test, is_public_visibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionParamSyntax {
    pub line: usize,
    pub function_name: String,
    pub param_name: String,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionReturnSyntax {
    pub line: usize,
    pub function_name: String,
    pub type_text: String,
    pub application_error_boundary: Option<String>,
    pub is_test_context: bool,
}

pub(crate) fn public_function_param_syntax(item: &syn::Item) -> Vec<RustFunctionParamSyntax> {
    let syn::Item::Fn(item_fn) = item else {
        return Vec::new();
    };
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    let function_name = item_fn.sig.ident.to_string();
    let is_test_context = attrs_have_cfg_test(&item_fn.attrs);
    item_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                return None;
            };
            Some(RustFunctionParamSyntax {
                line: pat_ident.span().start().line.max(1),
                function_name: function_name.clone(),
                param_name: pat_ident.ident.to_string(),
                type_text: pat_type.ty.to_token_stream().to_string(),
                primitive_contract_type: primitive_contract_type_name(&pat_type.ty),
                is_test_context,
            })
        })
        .collect()
}

pub(crate) fn public_function_return_syntax(item: &syn::Item) -> Option<RustFunctionReturnSyntax> {
    let syn::Item::Fn(item_fn) = item else {
        return None;
    };
    if !is_public_visibility(&item_fn.vis) {
        return None;
    }
    let syn::ReturnType::Type(_, return_type) = &item_fn.sig.output else {
        return None;
    };
    Some(RustFunctionReturnSyntax {
        line: item_fn.sig.ident.span().start().line.max(1),
        function_name: item_fn.sig.ident.to_string(),
        type_text: return_type.to_token_stream().to_string(),
        application_error_boundary: application_error_return_type(return_type),
        is_test_context: attrs_have_cfg_test(&item_fn.attrs),
    })
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

fn application_error_return_type(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let terminal = type_path.path.segments.last()?;
    if terminal.ident != "Result" {
        return None;
    }
    let path_text = path_segments_text(&type_path.path);
    if is_application_result_path(&path_text) {
        return Some(path_text);
    }
    let err_type = result_error_type(terminal)?;
    application_error_type_name(err_type).map(|err_name| format!("Result<_, {err_name}>"))
}

fn result_error_type(segment: &syn::PathSegment) -> Option<&syn::Type> {
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let mut types = args.args.iter().filter_map(|arg| {
        let syn::GenericArgument::Type(ty) = arg else {
            return None;
        };
        Some(ty)
    });
    types.next()?;
    types.next()
}

fn application_error_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) if is_application_error_path(&type_path.path) => {
            Some(path_segments_text(&type_path.path))
        }
        syn::Type::Path(type_path) if is_boxed_dyn_error_path(type_path) => {
            Some("Box<dyn Error>".to_owned())
        }
        syn::Type::TraitObject(trait_object) if trait_object_contains_error(trait_object) => {
            Some("dyn Error".to_owned())
        }
        _ => None,
    }
}

fn is_application_result_path(path_text: &str) -> bool {
    matches!(
        path_text,
        "anyhow::Result" | "eyre::Result" | "color_eyre::Result" | "color_eyre::eyre::Result"
    )
}

fn is_application_error_path(path: &syn::Path) -> bool {
    matches!(
        path_segments_text(path).as_str(),
        "anyhow::Error"
            | "eyre::Report"
            | "eyre::Error"
            | "color_eyre::Report"
            | "color_eyre::eyre::Report"
    )
}

fn is_boxed_dyn_error_path(type_path: &syn::TypePath) -> bool {
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Box" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    args.args.iter().any(|arg| {
        let syn::GenericArgument::Type(syn::Type::TraitObject(trait_object)) = arg else {
            return false;
        };
        trait_object_contains_error(trait_object)
    })
}

fn trait_object_contains_error(trait_object: &syn::TypeTraitObject) -> bool {
    trait_object.bounds.iter().any(|bound| {
        let syn::TypeParamBound::Trait(trait_bound) = bound else {
            return false;
        };
        trait_path_is_error_boundary(&trait_bound.path)
    })
}

fn trait_path_is_error_boundary(path: &syn::Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "Error")
        && (path.segments.len() == 1 || path_segments_text(path) == "std::error::Error")
}

fn path_segments_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
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
