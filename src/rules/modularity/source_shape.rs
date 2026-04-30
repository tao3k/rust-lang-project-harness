//! Source file shape and owner-boundary policies.

use std::collections::BTreeMap;

use syn::{Item, UseTree};

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::rules::display_path;
use crate::{RustHarnessFinding, RustHarnessRule};

use super::support::{
    count_effective_code_lines, count_implementation_items, count_public_items, item_span_line,
};
use super::{
    MAX_SOURCE_EFFECTIVE_LINES, MIN_SOURCE_IMPLEMENTATION_ITEMS, MIN_SOURCE_PUBLIC_ITEMS,
    RUST_MOD_R002, RUST_MOD_R003,
};

pub(super) fn source_file_bloat_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let effective_lines = count_effective_code_lines(&module.source);
    if effective_lines < MAX_SOURCE_EFFECTIVE_LINES {
        return Vec::new();
    }
    let public_items = count_public_items(items);
    let implementation_items = count_implementation_items(items);
    if public_items < MIN_SOURCE_PUBLIC_ITEMS
        && implementation_items < MIN_SOURCE_IMPLEMENTATION_ITEMS
    {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R002];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} carries {effective_lines} effective lines, {public_items} public items, and {implementation_items} top-level implementation items.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        None,
        "split this source file by responsibility",
    )]
}

pub(super) fn deep_relative_import_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_MOD_R003];
    items
        .iter()
        .filter_map(|item| match item {
            Item::Use(item_use) if use_tree_contains_super_super(&item_use.tree) => {
                let line_number = item_span_line(item);
                Some(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} uses deep relative import `super::super`.",
                        display_path(&module.report.path)
                    ),
                    path_line_location(&module.report.path, line_number),
                    source_line(&module.source, line_number),
                    "replace deep relative import with a clearer owner boundary",
                ))
            }
            _ => None,
        })
        .collect()
}

fn use_tree_contains_super_super(tree: &UseTree) -> bool {
    let mut segments = Vec::new();
    use_tree_contains_super_super_with_prefix(tree, &mut segments)
}

fn use_tree_contains_super_super_with_prefix(tree: &UseTree, segments: &mut Vec<String>) -> bool {
    match tree {
        UseTree::Path(path) => {
            segments.push(path.ident.to_string());
            let contains = has_super_super(segments)
                || use_tree_contains_super_super_with_prefix(&path.tree, segments);
            segments.pop();
            contains
        }
        UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_contains_super_super_with_prefix(item, segments)),
        UseTree::Name(name) => {
            segments.push(name.ident.to_string());
            let contains = has_super_super(segments);
            segments.pop();
            contains
        }
        UseTree::Rename(rename) => {
            segments.push(rename.ident.to_string());
            let contains = has_super_super(segments);
            segments.pop();
            contains
        }
        UseTree::Glob(_) => has_super_super(segments),
    }
}

fn has_super_super(segments: &[String]) -> bool {
    segments
        .windows(2)
        .any(|window| window[0] == "super" && window[1] == "super")
}
