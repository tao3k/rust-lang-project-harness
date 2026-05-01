//! Reasoning-tree reachability and branch-shape policies.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::rules::{display_path, is_under_any_dir};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::support::{is_special_entrypoint_name, normalize_path};
use super::{RUST_MOD_R007, RUST_MOD_R008, RUST_MOD_R009};

pub(super) fn module_source_shadow_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let source_files = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, &scope.source_paths))
        .map(|module| module.report.path.clone())
        .collect::<BTreeSet<_>>();
    let rule = &rules[RUST_MOD_R007];
    let mut reported = BTreeSet::<PathBuf>::new();
    let mut findings = Vec::new();
    for mod_rs in source_files
        .iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("mod.rs"))
    {
        let Some(parent) = mod_rs.parent() else {
            continue;
        };
        let Some(module_name) = parent.file_name() else {
            continue;
        };
        let Some(grandparent) = parent.parent() else {
            continue;
        };
        let sibling_file = grandparent.join(format!("{}.rs", module_name.to_string_lossy()));
        if !source_files.contains(&sibling_file) || !reported.insert(mod_rs.clone()) {
            continue;
        }
        findings.push(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} and {} both define the same Rust module source.",
                display_path(&sibling_file),
                display_path(mod_rs)
            ),
            file_location(mod_rs),
            None,
            "choose one module source layout for this owner",
        ));
    }
    findings
}

pub(super) fn orphan_source_module_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let source_files = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, &scope.source_paths))
        .map(|module| module.report.path.clone())
        .collect::<BTreeSet<_>>();
    if source_files.is_empty() {
        return Vec::new();
    }
    let modules_by_path = modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let mut reachable = BTreeSet::new();
    let mut stack = source_files
        .iter()
        .filter(|path| is_module_tree_root(scope, path))
        .cloned()
        .collect::<Vec<_>>();
    while let Some(path) = stack.pop() {
        if !reachable.insert(path.clone()) {
            continue;
        }
        let Some(module) = modules_by_path.get(&path) else {
            continue;
        };
        for child_path in external_child_module_paths(module, &source_files) {
            if !reachable.contains(&child_path) {
                stack.push(child_path);
            }
        }
    }
    let rule = &rules[RUST_MOD_R009];
    source_files
        .difference(&reachable)
        .filter(|path| !is_module_tree_root(scope, path))
        .map(|path| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} is not reachable from a crate or binary module tree.",
                    display_path(path)
                ),
                file_location(path),
                None,
                "attach this file with a parent mod declaration or remove it",
            )
        })
        .collect()
}

pub(super) fn inline_source_module_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if is_special_entrypoint_name(&module.report.path) {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R008];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| item.module.as_ref())
        .filter(|item_mod| item_mod.is_inline && !item_mod.is_cfg_test)
        .map(|item_mod| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains inline module `{}`.",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, item_mod.line),
                source_line(&module.source, item_mod.line),
                "move this inline module into its own source file",
            )
        })
        .collect()
}

fn external_child_module_paths(
    module: &ParsedRustModule,
    source_files: &BTreeSet<PathBuf>,
) -> Vec<PathBuf> {
    let module_path = &module.report.path;
    let mut paths = Vec::new();
    for item in &module.syntax_facts.top_level_items {
        if let Some(include_target) = &item.include_target {
            let include_path = normalize_path(
                module_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(include_target),
            );
            if source_files.contains(&include_path) {
                paths.push(include_path);
            }
        }
        let Some(item_mod) = &item.module else {
            continue;
        };
        if item_mod.is_inline || item_mod.is_cfg_test {
            continue;
        }
        if let Some(path_value) = &item_mod.path_attr {
            let resolved = normalize_path(
                module_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(path_value),
            );
            if source_files.contains(&resolved) {
                paths.push(resolved);
            }
            continue;
        }
        let base = child_module_base_dir(module_path);
        let name = &item_mod.ident;
        let file_form = base.join(format!("{name}.rs"));
        if source_files.contains(&file_form) {
            paths.push(file_form);
        }
        let mod_form = base.join(name).join("mod.rs");
        if source_files.contains(&mod_form) {
            paths.push(mod_form);
        }
    }
    paths
}

fn child_module_base_dir(module_path: &Path) -> PathBuf {
    let parent = module_path.parent().unwrap_or_else(|| Path::new(""));
    if is_special_entrypoint_name(module_path) {
        return parent.to_path_buf();
    }
    let Some(stem) = module_path.file_stem() else {
        return parent.to_path_buf();
    };
    parent.join(stem)
}

fn is_module_tree_root(scope: &RustProjectHarnessScope, path: &Path) -> bool {
    scope.source_paths.iter().any(|source_root| {
        if path == source_root.join("lib.rs") || path == source_root.join("main.rs") {
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
