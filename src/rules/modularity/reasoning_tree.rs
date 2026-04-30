//! Reasoning-tree reachability and branch-shape policies.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::Item;

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::rules::{display_path, has_cfg_test, is_under_any_dir};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::support::{is_special_entrypoint_name, item_span_line, normalize_path, path_attr_value};
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
        let Some(syntax) = &module.syntax else {
            continue;
        };
        for child_path in external_child_module_paths(&path, &syntax.items, &source_files) {
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
    items: &[Item],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if is_special_entrypoint_name(&module.report.path) {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R008];
    let mut findings = Vec::new();
    collect_inline_source_module_findings(module, items, rule, &mut findings);
    findings
}

fn collect_inline_source_module_findings(
    module: &ParsedRustModule,
    items: &[Item],
    rule: &RustHarnessRule,
    findings: &mut Vec<RustHarnessFinding>,
) {
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        if item_mod.content.is_none() || has_cfg_test(&item_mod.attrs) {
            continue;
        }
        let line = item_span_line(item);
        findings.push(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} contains inline module `{}`.",
                display_path(&module.report.path),
                item_mod.ident
            ),
            path_line_location(&module.report.path, line),
            source_line(&module.source, line),
            "move this inline module into its own source file",
        ));
    }
}

fn external_child_module_paths(
    module_path: &Path,
    items: &[Item],
    source_files: &BTreeSet<PathBuf>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        if item_mod.content.is_some() || has_cfg_test(&item_mod.attrs) {
            continue;
        }
        if let Some(path_value) = path_attr_value(&item_mod.attrs) {
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
        let name = item_mod.ident.to_string();
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
