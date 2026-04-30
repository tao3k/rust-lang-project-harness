//! Special Rust entrypoint and facade policies.

use std::collections::BTreeMap;
use std::path::Path;

use syn::Item;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::display_path;
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::support::{item_kind, item_span_line};
use super::{RUST_MOD_R001, RUST_MOD_R004, RUST_MOD_R005, RUST_MOD_R006};

pub(super) fn crate_facade_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module
        .report
        .path
        .file_name()
        .and_then(|name| name.to_str())
        != Some("lib.rs")
    {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R004];
    items
        .iter()
        .filter(|item| !is_crate_facade_item(item))
        .map(|item| {
            let line = item_span_line(item);
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside lib.rs.",
                    display_path(&module.report.path),
                    item_kind(item)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "move facade implementation out of lib.rs into an owned module",
            )
        })
        .collect()
}

pub(super) fn binary_entrypoint_findings(
    scope: &RustProjectHarnessScope,
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !is_binary_entrypoint_file(scope, &module.report.path) {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R005];
    items
        .iter()
        .filter(|item| !is_binary_entrypoint_item(item))
        .map(|item| {
            let line = item_span_line(item);
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside a binary entrypoint.",
                    display_path(&module.report.path),
                    item_kind(item)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "move binary implementation out of the entrypoint into an owned module",
            )
        })
        .collect()
}

pub(super) fn build_script_entrypoint_findings(
    scope: &RustProjectHarnessScope,
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !is_build_script_entrypoint_file(scope, &module.report.path) {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R006];
    items
        .iter()
        .filter(|item| !is_build_script_entrypoint_item(item))
        .map(|item| {
            let line = item_span_line(item);
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside build.rs.",
                    display_path(&module.report.path),
                    item_kind(item)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "move build script implementation out of build.rs into a build dependency",
            )
        })
        .collect()
}

pub(super) fn interface_mod_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if module
        .report
        .path
        .file_name()
        .and_then(|name| name.to_str())
        != Some("mod.rs")
    {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R001];
    items
        .iter()
        .filter(|item| !is_interface_item(item))
        .map(|item| {
            let line = item_span_line(item);
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside mod.rs.",
                    display_path(&module.report.path),
                    item_kind(item)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "move implementation out of mod.rs into an owned leaf module",
            )
        })
        .collect()
}

pub(super) fn is_package_entrypoint_file(scope: &RustProjectHarnessScope, path: &Path) -> bool {
    scope
        .package_paths
        .iter()
        .any(|entrypoint| entrypoint == path)
}

fn is_interface_item(item: &Item) -> bool {
    match item {
        Item::Mod(item_mod) => item_mod.content.is_none(),
        Item::Use(_) => true,
        _ => false,
    }
}

fn is_crate_facade_item(item: &Item) -> bool {
    match item {
        Item::Mod(item_mod) => item_mod.content.is_none(),
        Item::Use(_) => true,
        _ => false,
    }
}

fn is_binary_entrypoint_item(item: &Item) -> bool {
    match item {
        Item::Fn(item_fn) => item_fn.sig.ident == "main",
        Item::Use(_) => true,
        _ => false,
    }
}

fn is_build_script_entrypoint_item(item: &Item) -> bool {
    match item {
        Item::Fn(item_fn) => item_fn.sig.ident == "main",
        Item::Use(_) => true,
        _ => false,
    }
}

fn is_build_script_entrypoint_file(scope: &RustProjectHarnessScope, path: &Path) -> bool {
    is_package_entrypoint_file(scope, path)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "build.rs")
}

fn is_binary_entrypoint_file(scope: &RustProjectHarnessScope, path: &Path) -> bool {
    scope.source_paths.iter().any(|source_root| {
        if path == source_root.join("main.rs") {
            return true;
        }
        let Ok(relative) = path.strip_prefix(source_root) else {
            return false;
        };
        let components = relative
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>();
        matches!(
            components.as_slice(),
            [first, _] if first.as_ref() == "bin"
        ) || matches!(
            components.as_slice(),
            [first, _, file] if first.as_ref() == "bin" && file.as_ref() == "main.rs"
        )
    })
}
