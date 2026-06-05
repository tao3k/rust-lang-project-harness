//! Capture-node ABI helpers for Rust native tree-sitter projection.

use syn::spanned::Spanned;

pub(super) fn capture_field_for_projection(capture: &str, fields: &[String]) -> String {
    let preferred = if capture.ends_with(".method") {
        "method"
    } else if capture.starts_with("call.") {
        "function"
    } else if capture.ends_with(".name") {
        "name"
    } else if capture.ends_with(".target") {
        "target"
    } else {
        "item"
    };
    fields
        .iter()
        .find(|field| field.as_str() == preferred)
        .map_or_else(|| preferred.to_string(), Clone::clone)
}

pub(super) fn capture_node_for_item(
    parent_node: &'static str,
    item: &syn::Item,
    capture_field: &str,
    query_node_types: &[String],
) -> &'static str {
    if matches!(capture_field, "name" | "target") {
        let candidates: &[&str] = match item {
            syn::Item::Enum(_)
            | syn::Item::Impl(_)
            | syn::Item::Struct(_)
            | syn::Item::Trait(_)
            | syn::Item::TraitAlias(_)
            | syn::Item::Type(_) => &["type_identifier", "identifier", "scoped_type_identifier"],
            syn::Item::Use(_) => &["scoped_identifier", "identifier"],
            _ => &["identifier", "type_identifier", "scoped_identifier"],
        };
        return requested_node_or_default(query_node_types, candidates, candidates[0]);
    }
    parent_node
}

pub(super) fn capture_node_for_call(
    capture: &str,
    target: &str,
    query_node_types: &[String],
) -> &'static str {
    let candidates: &[&str] = if capture.ends_with(".method") {
        &["field_identifier", "identifier"]
    } else if target.contains("::") {
        &["scoped_identifier", "identifier", "field_identifier"]
    } else {
        &["identifier", "scoped_identifier", "field_identifier"]
    };
    requested_node_or_default(query_node_types, candidates, candidates[0])
}

pub(super) fn capture_line_range_for_item(
    item: &syn::Item,
    capture_field: &str,
    code_line: usize,
    item_start_line: usize,
    item_end_line: usize,
) -> (usize, usize) {
    if matches!(capture_field, "name" | "target")
        && let Some(span) = item_capture_span(item)
    {
        return span_line_range(span, code_line, item_start_line, item_end_line);
    }
    if capture_field == "item" {
        return (item_start_line, item_end_line);
    }
    (code_line, code_line)
}

fn item_capture_span(item: &syn::Item) -> Option<proc_macro2::Span> {
    match item {
        syn::Item::Const(item) => Some(item.ident.span()),
        syn::Item::Enum(item) => Some(item.ident.span()),
        syn::Item::ExternCrate(item) => Some(item.ident.span()),
        syn::Item::Fn(item) => Some(item.sig.ident.span()),
        syn::Item::Impl(item) => Some(item.self_ty.span()),
        syn::Item::Macro(item) => item.ident.as_ref().map(|ident| ident.span()).or_else(|| {
            item.mac
                .path
                .segments
                .last()
                .map(|segment| segment.ident.span())
        }),
        syn::Item::Mod(item) => Some(item.ident.span()),
        syn::Item::Static(item) => Some(item.ident.span()),
        syn::Item::Struct(item) => Some(item.ident.span()),
        syn::Item::Trait(item) => Some(item.ident.span()),
        syn::Item::TraitAlias(item) => Some(item.ident.span()),
        syn::Item::Type(item) => Some(item.ident.span()),
        syn::Item::Use(item) => Some(item.tree.span()),
        _ => None,
    }
}

fn span_line_range(
    span: proc_macro2::Span,
    fallback_line: usize,
    item_start_line: usize,
    item_end_line: usize,
) -> (usize, usize) {
    let start = span.start().line.max(1);
    let end = span.end().line.max(start);
    if start < item_start_line || start > item_end_line {
        return (fallback_line, fallback_line);
    }
    (start, end.min(item_end_line).max(start))
}

fn requested_node_or_default(
    query_node_types: &[String],
    candidates: &[&'static str],
    default: &'static str,
) -> &'static str {
    candidates
        .iter()
        .copied()
        .find(|candidate| {
            query_node_types
                .iter()
                .any(|node_type| node_type.as_str() == *candidate)
        })
        .unwrap_or(default)
}
