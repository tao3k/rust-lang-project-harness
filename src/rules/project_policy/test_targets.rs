//! Cargo test target policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{
    CargoManifestFacts, ParsedRustModule, RustTopLevelItemSyntax, file_location,
    path_line_location, rust_source_path_facts, source_line,
};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use super::config::{LayoutPolicy, is_allowed_test_suite_path};
use super::support::display_project_path;
use super::{RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008, RUST_PROJ_R009};

pub(super) fn test_target_gate_findings(
    project_root: &Path,
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R006];
    for parsed in cargo_test_targets {
        if parsed
            .syntax_facts
            .contains_invocation_named(ROOT_HARNESS_GATE_INVOCATIONS)
        {
            continue;
        }
        findings.push(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} does not mount the Rust project harness gate.",
                display_project_path(project_root, &parsed.report.path)
            ),
            file_location(&parsed.report.path),
            None,
            "add rust_project_harness_gate!() to this Cargo test target",
        ));
    }
    findings
}

pub(super) fn library_cargo_test_gate_findings(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let Some(lib_path) = library_target_path(scope, modules) else {
        return Vec::new();
    };
    if !project_uses_harness_gate(cargo_manifest, modules)
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
        "add #[cfg(test)] rust_lang_project_harness::rust_project_harness_cargo_test_gate!()",
    )]
}

fn library_target_path(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
) -> Option<std::path::PathBuf> {
    modules.iter().find_map(|module| {
        let path_facts = rust_source_path_facts(
            &scope.project_root,
            &scope.source_paths,
            &scope.package_paths,
            &module.report.path,
        );
        path_facts
            .is_crate_facade
            .then(|| module.report.path.clone())
    })
}

pub(super) fn test_target_aggregate_findings(
    project_root: &Path,
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R007];
    for parsed in cargo_test_targets {
        for item in parsed
            .syntax_facts
            .top_level_items
            .iter()
            .filter(|item| !is_test_target_aggregate_item_syntax(item))
        {
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains top-level implementation item `{}`.",
                    display_project_path(project_root, &parsed.report.path),
                    item.kind
                ),
                path_line_location(&parsed.report.path, item.line),
                source_line(&parsed.source, item.line),
                "move test implementation into a suite module and mount it from the root target",
            ));
        }
    }
    findings
}

pub(super) fn test_target_module_mount_findings(
    project_root: &Path,
    cargo_test_targets: &[ParsedRustModule],
    policy: &LayoutPolicy,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R008];
    for parsed in cargo_test_targets {
        for item_mod in parsed
            .syntax_facts
            .top_level_items
            .iter()
            .filter_map(|item| item.module.as_ref())
            .filter(|item_mod| !item_mod.is_inline)
        {
            let Some(path_value) = item_mod.path_attr.as_deref() else {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} declares root module `{}` without an explicit #[path].",
                        display_project_path(project_root, &parsed.report.path),
                        item_mod.ident
                    ),
                    path_line_location(&parsed.report.path, item_mod.line),
                    source_line(&parsed.source, item_mod.line),
                    "mount this root test module with #[path = \"suite/file.rs\"]",
                ));
                continue;
            };
            let Some(resolved) = item_mod.resolved_path_attr.as_ref() else {
                continue;
            };
            let project_relative = resolved.strip_prefix(project_root).unwrap_or(resolved);
            if !resolved.exists() || !is_allowed_test_suite_path(project_relative, policy) {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} mounts `{path_value}`, but root test modules must resolve under an allowed tests suite directory.",
                        display_project_path(project_root, &parsed.report.path)
                    ),
                    path_line_location(&parsed.report.path, item_mod.line),
                    source_line(&parsed.source, item_mod.line),
                    "point this root test module at tests/unit, tests/integration, or a documented suite",
                ));
            }
        }
    }
    findings
}

fn project_uses_harness_gate(
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
) -> bool {
    cargo_manifest.references_harness || modules.iter().any(module_syntax_contains_any_harness_gate)
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
        .syntax_facts
        .contains_macro_named(ANY_HARNESS_GATE_MACROS)
}

fn module_syntax_contains_cargo_test_gate(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .contains_macro_named(SOURCE_CARGO_TEST_GATE_MACROS)
}

const ROOT_HARNESS_GATE_INVOCATIONS: &[&str] = &[
    "rust_project_harness_gate",
    "rust_project_harness_cargo_test_gate",
    "rust_project_harness_source_gate",
    "assert_rust_project_harness_clean",
    "run_rust_project_harness",
    "crate_testing_gate",
    "crate_test_policy_harness",
    "crate_test_policy_source_harness",
    "crate_testing_source_gate",
];

const ANY_HARNESS_GATE_MACROS: &[&str] = &[
    "rust_project_harness_gate",
    "rust_project_harness_cargo_test_gate",
    "rust_project_harness_source_gate",
    "crate_testing_gate",
    "crate_test_policy_harness",
    "crate_test_policy_source_harness",
    "crate_testing_source_gate",
];

const SOURCE_CARGO_TEST_GATE_MACROS: &[&str] = &[
    "rust_project_harness_cargo_test_gate",
    "rust_project_harness_source_gate",
    "rust_project_harness_gate",
    "crate_test_policy_source_harness",
    "crate_testing_source_gate",
    "crate_testing_gate",
    "crate_test_policy_harness",
];

fn is_test_target_aggregate_item_syntax(item: &RustTopLevelItemSyntax) -> bool {
    item.is_macro || item.is_use || item.module.as_ref().is_some_and(|module| !module.is_inline)
}
