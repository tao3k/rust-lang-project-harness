//! Build-script harness gate policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{CargoManifestFacts, ParsedRustModule, file_location};
use crate::{RustHarnessFinding, RustHarnessRule};

use super::RUST_PROJ_R012;
use super::support::display_project_path;

pub(super) fn build_gate_findings(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let build_script_path = project_root.join("build.rs");
    let build_script = root_build_script_module(project_root, modules);
    let build_script_exists = build_script.is_some() || build_script_path.exists();
    let has_build_gate_call = build_script.is_some_and(module_contains_build_gate_call);
    let harness_enabled = cargo_manifest.references_harness || has_build_gate_call;

    if !harness_enabled || project_has_complete_build_gate(project_root, cargo_manifest, modules) {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R012];
    if !cargo_manifest.references_harness_build_dependency && !build_script_exists {
        return vec![RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} enables the harness without a cargo-check build gate.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add rust-lang-project-harness under [build-dependencies] and add a thin root build.rs that calls rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(...)",
        )];
    }

    if cargo_manifest.references_harness_build_dependency && !build_script_exists {
        return vec![RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} declares a harness build-dependency but does not provide a root build.rs gate.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add a thin root build.rs that calls rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(...)",
        )];
    }

    if build_script_exists && !has_build_gate_call {
        return vec![RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} exists in a harness-enabled project but does not mount the build-time harness gate.",
                display_project_path(project_root, &build_script_path)
            ),
            file_location(&build_script_path),
            None,
            "add the harness to [build-dependencies] and call rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(...) from root build.rs",
        )];
    }

    if has_build_gate_call && !cargo_manifest.references_harness_build_dependency {
        return vec![RustHarnessFinding::from_rule(
            rule,
            format!(
                "{} calls the build-time harness gate but Cargo.toml does not declare the harness as a build-dependency.",
                display_project_path(project_root, &build_script_path)
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add rust-lang-project-harness under [build-dependencies] so Cargo can compile the build gate",
        )];
    }

    Vec::new()
}

pub(super) fn project_has_complete_build_gate(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
) -> bool {
    cargo_manifest.references_harness_build_dependency
        && root_build_script_module(project_root, modules)
            .is_some_and(module_contains_build_gate_call)
}

pub(super) fn root_build_script_module<'a>(
    project_root: &Path,
    modules: &'a [ParsedRustModule],
) -> Option<&'a ParsedRustModule> {
    let build_script_path = project_root.join("build.rs");
    modules
        .iter()
        .find(|module| same_path(&module.report.path, &build_script_path))
}

pub(super) fn module_contains_build_gate_call(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .contains_function_call_named(BUILD_GATE_FUNCTIONS)
}

pub(super) fn module_default_build_gate_call_lines(
    module: &ParsedRustModule,
) -> impl Iterator<Item = usize> + '_ {
    module
        .syntax_facts
        .function_calls
        .iter()
        .filter(|invocation| {
            DEFAULT_BUILD_GATE_FUNCTIONS.contains(&invocation.terminal_name.as_str())
        })
        .map(|invocation| invocation.line)
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

const BUILD_GATE_FUNCTIONS: &[&str] = &[
    "assert_rust_project_harness_build_clean",
    "assert_rust_project_harness_build_clean_with_config",
    "assert_rust_project_harness_build_clean_from_env",
    "assert_rust_project_harness_build_clean_from_env_with_config",
    "assert_rust_project_harness_cargo_check_clean",
    "assert_rust_project_harness_cargo_check_clean_with_config",
    "assert_rust_project_harness_cargo_check_clean_from_env",
    "assert_rust_project_harness_cargo_check_clean_from_env_with_config",
    "assert_rust_project_harness_downstream_policy",
    "assert_rust_project_harness_downstream_policy_from_env",
];

const DEFAULT_BUILD_GATE_FUNCTIONS: &[&str] = &[
    "assert_rust_project_harness_build_clean",
    "assert_rust_project_harness_build_clean_from_env",
    "assert_rust_project_harness_cargo_check_clean",
    "assert_rust_project_harness_cargo_check_clean_from_env",
];
