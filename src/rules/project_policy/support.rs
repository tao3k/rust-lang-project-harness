//! Shared project-policy helpers.

use std::fs;
use std::path::{Component, Path, PathBuf};

use quote::ToTokens;
use syn::{Attribute, Item, Meta};

use crate::rules::display_path;

pub(super) fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
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

pub(super) fn resolve_path_attr(source_file: &Path, path_value: &str) -> PathBuf {
    normalize_path(
        source_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path_value),
    )
}

fn normalize_path(path: PathBuf) -> PathBuf {
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

pub(super) fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if is_rust_file(&path) {
            files.push(path);
        }
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

pub(super) fn count_test_functions(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Fn(item_fn) => usize::from(
                item_fn
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("test")),
            ),
            Item::Mod(item_mod) => item_mod
                .content
                .as_ref()
                .map_or(0, |(_, nested_items)| count_test_functions(nested_items)),
            _ => 0,
        })
        .sum()
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

pub(super) fn display_project_path(project_root: &Path, path: &Path) -> String {
    display_path(path.strip_prefix(project_root).unwrap_or(path))
}
