//! Native Rust module and attribute facts.

use std::path::Path;

use syn::spanned::Spanned;

use super::facts::RustModuleDeclarationSyntax;
use crate::parser::resolve_rust_path_attr;

pub(super) fn module_declaration_syntax(
    item: &syn::Item,
    source_file: &Path,
) -> Option<RustModuleDeclarationSyntax> {
    let syn::Item::Mod(item_mod) = item else {
        return None;
    };
    Some(module_declaration_from_item_mod(item_mod, source_file))
}

pub(super) fn module_declaration_from_item_mod(
    item_mod: &syn::ItemMod,
    source_file: &Path,
) -> RustModuleDeclarationSyntax {
    let line = item_mod.attrs.first().map_or_else(
        || item_mod.span().start().line.max(1),
        |attr| attr.span().start().line.max(1),
    );
    let path_attr = path_attr_value(&item_mod.attrs);
    let resolved_path_attr = path_attr
        .as_deref()
        .map(|path_value| resolve_rust_path_attr(source_file, path_value));
    RustModuleDeclarationSyntax {
        line,
        ident: item_mod.ident.to_string(),
        path_attr,
        resolved_path_attr,
        is_inline: item_mod.content.is_some(),
        is_cfg_test: attrs_have_cfg_test(&item_mod.attrs),
    }
}

fn path_attr_value(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }
        let syn::Meta::NameValue(name_value) = &attr.meta else {
            return None;
        };
        let syn::Expr::Lit(expr_lit) = &name_value.value else {
            return None;
        };
        let syn::Lit::Str(lit_str) = &expr_lit.lit else {
            return None;
        };
        Some(lit_str.value())
    })
}

pub(super) fn attrs_have_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attribute_has_cfg_test)
}

pub(super) fn attrs_have_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || attr
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "test")
    })
}

pub(super) fn attrs_have_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}

pub(super) fn attribute_is_proc_macro_export(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("proc_macro")
        || attr.path().is_ident("proc_macro_attribute")
        || attr.path().is_ident("proc_macro_derive")
}

pub(super) fn attribute_is_cfg(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("cfg")
}

pub(super) fn attribute_is_cfg_feature(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map(|metas| metas.iter().any(meta_mentions_feature))
        .unwrap_or(false)
}

fn meta_mentions_feature(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::NameValue(name_value) => name_value.path.is_ident("feature"),
        syn::Meta::List(list) => {
            let Some(ident) = list.path.get_ident() else {
                return false;
            };
            if !(ident == "all" || ident == "any" || ident == "not") {
                return false;
            }
            list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .map(|metas| metas.iter().any(meta_mentions_feature))
            .unwrap_or(false)
        }
        syn::Meta::Path(_) => false,
    }
}

fn attribute_has_cfg_test(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map(|metas| {
            metas
                .iter()
                .any(|meta| matches!(meta, syn::Meta::Path(path) if path.is_ident("test")))
        })
        .unwrap_or(false)
}
