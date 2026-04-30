//! Cargo test target policy.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use syn::Item;
use syn::spanned::Spanned;

use crate::parser::ParsedRustModule;
use crate::parser::{file_location, path_line_location, source_line};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::config::{LayoutPolicy, is_allowed_test_suite_path};
use super::support::{
    display_project_path, is_rust_file, item_kind, path_attr_value, resolve_path_attr,
};
use super::{RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008, RUST_PROJ_R009};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CargoManifestToml {
    test: Vec<CargoTestTargetToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CargoTestTargetToml {
    path: String,
}

pub(super) fn test_target_gate_findings(
    project_root: &Path,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R006];
    for target in collect_test_target_files(project_root) {
        let Ok(content) = fs::read_to_string(&target) else {
            continue;
        };
        if file_contains_harness_gate(&content) {
            continue;
        }
        findings.push(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} does not mount the Rust project harness gate.",
                display_project_path(project_root, &target)
            ),
            file_location(target),
            None,
            "add rust_project_harness_gate!() to this Cargo test target",
        ));
    }
    findings
}

pub(super) fn library_cargo_test_gate_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let Some(lib_path) = scope
        .source_paths
        .iter()
        .map(|source_root| source_root.join("lib.rs"))
        .find(|path| path.exists())
    else {
        return Vec::new();
    };
    if !project_uses_harness_gate(&scope.project_root, modules)
        || source_tree_contains_cargo_test_gate(scope, modules)
    {
        return Vec::new();
    }
    let rule = &rules[RUST_PROJ_R009];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} is a library target in a harness-enabled project but does not mount a cargo-test harness gate.",
            display_project_path(&scope.project_root, &lib_path)
        ),
        file_location(lib_path),
        None,
        "add #[cfg(test)] xiuxian_harness_rust_lang_project::rust_project_harness_cargo_test_gate!()",
    )]
}

pub(super) fn test_target_aggregate_findings(
    project_root: &Path,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R007];
    for target in collect_test_target_files(project_root) {
        let Ok(content) = fs::read_to_string(&target) else {
            continue;
        };
        let Ok(syntax) = syn::parse_file(&content) else {
            continue;
        };
        for item in syntax
            .items
            .iter()
            .filter(|item| !is_test_target_aggregate_item(item))
        {
            let line = item.span().start().line.max(1);
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains top-level implementation item `{}`.",
                    display_project_path(project_root, &target),
                    item_kind(item)
                ),
                path_line_location(&target, line),
                source_line(&content, line),
                "move test implementation into a suite module and mount it from the root target",
            ));
        }
    }
    findings
}

pub(super) fn test_target_module_mount_findings(
    project_root: &Path,
    policy: &LayoutPolicy,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R008];
    for target in collect_test_target_files(project_root) {
        let Ok(content) = fs::read_to_string(&target) else {
            continue;
        };
        let Ok(syntax) = syn::parse_file(&content) else {
            continue;
        };
        for item_mod in syntax.items.iter().filter_map(|item| match item {
            Item::Mod(item_mod) if item_mod.content.is_none() => Some(item_mod),
            _ => None,
        }) {
            let line = item_mod.attrs.first().map_or_else(
                || item_mod.span().start().line.max(1),
                |attr| attr.span().start().line.max(1),
            );
            let Some(path_value) = path_attr_value(&item_mod.attrs) else {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} declares root module `{}` without an explicit #[path].",
                        display_project_path(project_root, &target),
                        item_mod.ident
                    ),
                    path_line_location(&target, line),
                    source_line(&content, line),
                    "mount this root test module with #[path = \"suite/file.rs\"]",
                ));
                continue;
            };
            let resolved = resolve_path_attr(&target, &path_value);
            let project_relative = resolved.strip_prefix(project_root).unwrap_or(&resolved);
            if !resolved.exists() || !is_allowed_test_suite_path(project_relative, policy) {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} mounts `{path_value}`, but root test modules must resolve under an allowed tests suite directory.",
                        display_project_path(project_root, &target)
                    ),
                    path_line_location(&target, line),
                    source_line(&content, line),
                    "point this root test module at tests/unit, tests/integration, or a documented suite",
                ));
            }
        }
    }
    findings
}

fn collect_test_target_files(project_root: &Path) -> Vec<PathBuf> {
    let mut targets = BTreeSet::new();
    let tests_dir = project_root.join("tests");
    if let Ok(entries) = fs::read_dir(&tests_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_rust_file(&path) {
                targets.insert(path);
            }
        }
    }
    let manifest_path = project_root.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&manifest_path)
        && let Ok(parsed) = toml::from_str::<CargoManifestToml>(&content)
    {
        for target in parsed.test {
            let target_path = target.path.trim();
            if !target_path.is_empty() {
                targets.insert(project_root.join(target_path));
            }
        }
    }
    targets.into_iter().collect()
}

fn file_contains_harness_gate(content: &str) -> bool {
    [
        "rust_project_harness_gate!(",
        "rust_project_harness_cargo_test_gate!(",
        "rust_project_harness_source_gate!(",
        "assert_rust_project_harness_clean(",
        "run_rust_project_harness(",
        "crate_testing_gate!(",
        "crate_test_policy_harness!(",
    ]
    .iter()
    .any(|needle| content.contains(needle))
}

fn project_uses_harness_gate(project_root: &Path, modules: &[ParsedRustModule]) -> bool {
    manifest_mentions_harness(project_root)
        || modules.iter().any(module_syntax_contains_any_harness_gate)
}

fn manifest_mentions_harness(project_root: &Path) -> bool {
    let Ok(content) = fs::read_to_string(project_root.join("Cargo.toml")) else {
        return false;
    };
    content.contains("xiuxian-harness-rust-lang-project")
        || content.contains("xiuxian_harness_rust_lang_project")
}

fn source_tree_contains_cargo_test_gate(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
) -> bool {
    modules.iter().any(|module| {
        scope
            .source_paths
            .iter()
            .any(|source_root| module.report.path.starts_with(source_root))
            && module_syntax_contains_cargo_test_gate(module)
    })
}

fn module_syntax_contains_any_harness_gate(module: &ParsedRustModule) -> bool {
    module
        .syntax
        .as_ref()
        .is_some_and(|syntax| items_contain_macro_gate(&syntax.items, ANY_HARNESS_GATE_MACROS))
}

fn module_syntax_contains_cargo_test_gate(module: &ParsedRustModule) -> bool {
    module.syntax.as_ref().is_some_and(|syntax| {
        items_contain_macro_gate(&syntax.items, SOURCE_CARGO_TEST_GATE_MACROS)
    })
}

fn items_contain_macro_gate(items: &[Item], macro_names: &[&str]) -> bool {
    items.iter().any(|item| match item {
        Item::Macro(item_macro) => macro_path_matches(&item_macro.mac.path, macro_names),
        Item::Mod(item_mod) => item_mod
            .content
            .as_ref()
            .is_some_and(|(_, items)| items_contain_macro_gate(items, macro_names)),
        _ => false,
    })
}

fn macro_path_matches(path: &syn::Path, macro_names: &[&str]) -> bool {
    let Some(segment) = path.segments.last() else {
        return false;
    };
    let ident = segment.ident.to_string();
    macro_names.contains(&ident.as_str())
}

const ANY_HARNESS_GATE_MACROS: &[&str] = &[
    "rust_project_harness_gate",
    "rust_project_harness_cargo_test_gate",
    "rust_project_harness_source_gate",
    "crate_testing_gate",
    "crate_test_policy_harness",
];

const SOURCE_CARGO_TEST_GATE_MACROS: &[&str] = &[
    "rust_project_harness_cargo_test_gate",
    "rust_project_harness_source_gate",
    "rust_project_harness_gate",
];

fn is_test_target_aggregate_item(item: &Item) -> bool {
    match item {
        Item::Macro(_) | Item::Use(_) => true,
        Item::Mod(item_mod) => item_mod.content.is_none(),
        _ => false,
    }
}
