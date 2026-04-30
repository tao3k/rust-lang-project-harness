//! Shared modularity rule helpers.

use std::path::{Component, Path, PathBuf};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Item, Meta, Visibility};

pub(super) fn item_span_line(item: &Item) -> usize {
    item.span().start().line.max(1)
}

pub(super) fn item_kind(item: &Item) -> &'static str {
    match item {
        Item::Const(_) => "const",
        Item::Enum(_) => "enum",
        Item::Fn(_) => "fn",
        Item::Impl(_) => "impl",
        Item::Macro(_) => "macro",
        Item::Mod(_) => "mod",
        Item::Static(_) => "static",
        Item::Struct(_) => "struct",
        Item::Trait(_) => "trait",
        Item::TraitAlias(_) => "trait_alias",
        Item::Type(_) => "type",
        Item::Union(_) => "union",
        Item::Use(_) => "use",
        _ => "item",
    }
}

pub(super) fn count_effective_code_lines(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("//")
                && !line.starts_with("/*")
                && !line.starts_with('*')
                && !line.starts_with("*/")
                && !line.starts_with("#[")
                && !line.starts_with("#![")
        })
        .count()
}

pub(super) fn count_public_items(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Const(item) => usize::from(is_public(&item.vis)),
            Item::Enum(item) => usize::from(is_public(&item.vis)),
            Item::Fn(item) => usize::from(is_public(&item.vis)),
            Item::Mod(item) => usize::from(is_public(&item.vis)),
            Item::Static(item) => usize::from(is_public(&item.vis)),
            Item::Struct(item) => usize::from(is_public(&item.vis)),
            Item::Trait(item) => usize::from(is_public(&item.vis)),
            Item::TraitAlias(item) => usize::from(is_public(&item.vis)),
            Item::Type(item) => usize::from(is_public(&item.vis)),
            Item::Union(item) => usize::from(is_public(&item.vis)),
            _ => 0,
        })
        .sum()
}

pub(super) fn count_implementation_items(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Const(_)
            | Item::Enum(_)
            | Item::Fn(_)
            | Item::Impl(_)
            | Item::Static(_)
            | Item::Struct(_)
            | Item::Trait(_)
            | Item::TraitAlias(_)
            | Item::Type(_)
            | Item::Union(_) => 1,
            _ => 0,
        })
        .sum()
}

pub(super) fn is_special_entrypoint_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "lib.rs" | "main.rs" | "mod.rs"))
}

pub(super) fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg") && attr.meta.to_token_stream().to_string().contains("test")
    })
}

pub(super) fn path_attr_value(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(name_value) = &attr.meta else {
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

pub(super) fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(_) | Component::Prefix(_) | Component::RootDir => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}
