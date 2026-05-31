//! Native Rust public signature facts.

use quote::ToTokens;
use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionParamSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub param_name: String,
    pub type_text: String,
    pub primitive_contract_type: Option<String>,
    pub flag_contract_type: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionReturnSyntax {
    pub line: usize,
    pub function_name: String,
    pub type_text: String,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub receiver: Option<String>,
    pub impl_type: Option<String>,
    pub trait_path: Option<String>,
    pub application_error_boundary: Option<String>,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustFunctionTupleApiSyntax {
    pub line: usize,
    pub function_line: usize,
    pub function_name: String,
    pub surface_name: String,
    pub type_text: String,
    pub element_contract_types: Vec<String>,
    pub is_test_context: bool,
}

pub(crate) fn public_function_param_syntax(item: &syn::Item) -> Vec<RustFunctionParamSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_param_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => public_impl_function_param_syntax(item_impl),
        _ => Vec::new(),
    }
}

pub(crate) fn public_function_return_syntax(item: &syn::Item) -> Vec<RustFunctionReturnSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_return_syntax(item_fn, false)
            .into_iter()
            .collect(),
        syn::Item::Impl(item_impl) => public_impl_function_return_syntax(item_impl),
        _ => Vec::new(),
    }
}

pub(crate) fn public_function_tuple_api_syntax(
    item: &syn::Item,
) -> Vec<RustFunctionTupleApiSyntax> {
    match item {
        syn::Item::Fn(item_fn) => public_item_function_tuple_api_syntax(item_fn, false),
        syn::Item::Impl(item_impl) => public_impl_function_tuple_api_syntax(item_impl),
        _ => Vec::new(),
    }
}

fn public_item_function_param_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustFunctionParamSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    signature_param_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    )
}

fn public_impl_function_param_syntax(item_impl: &syn::ItemImpl) -> Vec<RustFunctionParamSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            impl_method_is_public_api(item_impl, method).then_some(method)
        })
        .flat_map(|method| {
            signature_param_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            )
        })
        .collect()
}

fn signature_param_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
) -> Vec<RustFunctionParamSyntax> {
    let function_name = signature.ident.to_string();
    let function_line = signature.ident.span().start().line.max(1);
    signature
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
                function_line,
                function_name: function_name.clone(),
                param_name: pat_ident.ident.to_string(),
                type_text: pat_type.ty.to_token_stream().to_string(),
                primitive_contract_type: primitive_contract_type_name(&pat_type.ty),
                flag_contract_type: flag_contract_type_name(&pat_type.ty),
                is_test_context,
            })
        })
        .collect()
}

fn public_item_function_tuple_api_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Vec<RustFunctionTupleApiSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return Vec::new();
    }
    signature_tuple_api_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
    )
}

fn public_impl_function_tuple_api_syntax(
    item_impl: &syn::ItemImpl,
) -> Vec<RustFunctionTupleApiSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            impl_method_is_public_api(item_impl, method).then_some(method)
        })
        .flat_map(|method| {
            signature_tuple_api_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
            )
        })
        .collect()
}

fn signature_tuple_api_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
) -> Vec<RustFunctionTupleApiSyntax> {
    let function_name = signature.ident.to_string();
    let function_line = signature.ident.span().start().line.max(1);
    let mut facts = signature
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                return None;
            };
            let element_contract_types = tuple_api_contract_types(&pat_type.ty)?;
            Some(RustFunctionTupleApiSyntax {
                line: pat_ident.span().start().line.max(1),
                function_line,
                function_name: function_name.clone(),
                surface_name: format!("parameter `{}`", pat_ident.ident),
                type_text: pat_type.ty.to_token_stream().to_string(),
                element_contract_types,
                is_test_context,
            })
        })
        .collect::<Vec<_>>();

    if let syn::ReturnType::Type(_, return_type) = &signature.output
        && let Some(element_contract_types) = tuple_api_contract_types(return_type)
    {
        facts.push(RustFunctionTupleApiSyntax {
            line: function_line,
            function_line,
            function_name,
            surface_name: "return value".to_owned(),
            type_text: return_type.to_token_stream().to_string(),
            element_contract_types,
            is_test_context,
        });
    }

    facts
}

fn public_item_function_return_syntax(
    item_fn: &syn::ItemFn,
    inherited_test_context: bool,
) -> Option<RustFunctionReturnSyntax> {
    if !is_public_visibility(&item_fn.vis) {
        return None;
    };
    signature_return_syntax(
        &item_fn.sig,
        inherited_test_context || attrs_have_cfg_test(&item_fn.attrs),
        None,
        None,
    )
}

fn public_impl_function_return_syntax(item_impl: &syn::ItemImpl) -> Vec<RustFunctionReturnSyntax> {
    let inherited_test_context = attrs_have_cfg_test(&item_impl.attrs);
    let impl_type = Some(item_impl.self_ty.to_token_stream().to_string());
    let trait_path = item_impl
        .trait_
        .as_ref()
        .map(|(_, path, _)| path.to_token_stream().to_string());
    item_impl
        .items
        .iter()
        .filter_map(|impl_item| {
            let syn::ImplItem::Fn(method) = impl_item else {
                return None;
            };
            if !impl_method_is_public_api(item_impl, method) {
                return None;
            }
            signature_return_syntax(
                &method.sig,
                inherited_test_context || attrs_have_cfg_test(&method.attrs),
                impl_type.clone(),
                trait_path.clone(),
            )
        })
        .collect()
}

fn signature_return_syntax(
    signature: &syn::Signature,
    is_test_context: bool,
    impl_type: Option<String>,
    trait_path: Option<String>,
) -> Option<RustFunctionReturnSyntax> {
    let syn::ReturnType::Type(_, return_type) = &signature.output else {
        return None;
    };
    Some(RustFunctionReturnSyntax {
        line: signature.ident.span().start().line.max(1),
        function_name: signature.ident.to_string(),
        type_text: return_type.to_token_stream().to_string(),
        is_async: signature.asyncness.is_some(),
        is_unsafe: signature.unsafety.is_some(),
        receiver: signature_receiver(signature),
        impl_type,
        trait_path,
        application_error_boundary: application_error_return_type(return_type),
        is_test_context,
    })
}

fn impl_method_is_public_api(item_impl: &syn::ItemImpl, method: &syn::ImplItemFn) -> bool {
    item_impl.trait_.is_some() || is_public_visibility(&method.vis)
}

fn signature_receiver(signature: &syn::Signature) -> Option<String> {
    signature.inputs.iter().find_map(|arg| {
        let syn::FnArg::Receiver(receiver) = arg else {
            return None;
        };
        Some(
            match (receiver.reference.is_some(), receiver.mutability.is_some()) {
                (true, true) => "&mut-self".to_string(),
                (true, false) => "&self".to_string(),
                (false, true) => "mut-self".to_string(),
                (false, false) => "self".to_string(),
            },
        )
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

fn flag_contract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => flag_contract_path_name(type_path),
        syn::Type::Reference(reference) => {
            flag_contract_type_name(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => None,
    }
}

fn tuple_api_contract_types(ty: &syn::Type) -> Option<Vec<String>> {
    match ty {
        syn::Type::Tuple(tuple) => semantic_tuple_contract_types(&tuple.elems),
        syn::Type::Paren(paren) => tuple_api_contract_types(&paren.elem),
        syn::Type::Group(group) => tuple_api_contract_types(&group.elem),
        syn::Type::Reference(reference) => tuple_api_contract_types(&reference.elem),
        syn::Type::Path(type_path) => wrapper_tuple_contract_types(type_path),
        _ => None,
    }
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

fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg") && attr.to_token_stream().to_string().contains("test")
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
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
