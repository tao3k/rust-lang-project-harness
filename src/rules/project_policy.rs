//! Project-level Rust test policy rules.

mod catalog;
mod config;
mod source_tests;
mod support;
mod test_bloat;
mod test_layout;
mod test_targets;

use crate::parser::{
    ParsedRustModule, parse_cargo_manifest, parse_cargo_test_targets, rust_reasoning_tree_facts,
};
use crate::{RustHarnessFinding, RustHarnessRule, RustProjectHarnessScope};

use catalog::rules_by_id;
use config::load_layout_policy;
use source_tests::source_test_mount_findings;
use test_bloat::test_leaf_bloat_findings;
use test_layout::test_layout_findings;
use test_targets::{
    library_cargo_test_gate_findings, test_target_aggregate_findings, test_target_gate_findings,
    test_target_module_mount_findings,
};

pub(crate) const PACK_ID: &str = "rust.project_policy";
pub(crate) const RUST_PROJ_R001: &str = "RUST-PROJ-R001";
pub(crate) const RUST_PROJ_R002: &str = "RUST-PROJ-R002";
pub(crate) const RUST_PROJ_R003: &str = "RUST-PROJ-R003";
pub(crate) const RUST_PROJ_R004: &str = "RUST-PROJ-R004";
pub(crate) const RUST_PROJ_R005: &str = "RUST-PROJ-R005";
pub(crate) const RUST_PROJ_R006: &str = "RUST-PROJ-R006";
pub(crate) const RUST_PROJ_R007: &str = "RUST-PROJ-R007";
pub(crate) const RUST_PROJ_R008: &str = "RUST-PROJ-R008";
pub(crate) const RUST_PROJ_R009: &str = "RUST-PROJ-R009";

pub(crate) const MAX_UNIT_TEST_EFFECTIVE_LINES: usize = 260;
pub(crate) const MIN_UNIT_TEST_FUNCTIONS: usize = 8;
pub(crate) const MAX_INTEGRATION_TEST_EFFECTIVE_LINES: usize = 420;
pub(crate) const MIN_INTEGRATION_TEST_FUNCTIONS: usize = 12;

/// Return compact metadata for Rust project-policy rules.
#[must_use]
pub fn rust_project_policy_rules() -> Vec<RustHarnessRule> {
    rules_by_id().into_values().collect()
}

pub(crate) fn evaluate(
    scope: Option<&RustProjectHarnessScope>,
    modules: &[ParsedRustModule],
) -> Vec<RustHarnessFinding> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let rules = rules_by_id();
    let mut findings = Vec::new();
    let policy = load_layout_policy(&scope.project_root);
    let cargo_manifest = parse_cargo_manifest(&scope.project_root);
    let cargo_test_targets = parse_cargo_test_targets(&scope.project_root, &cargo_manifest);
    let reasoning_tree = rust_reasoning_tree_facts(scope, modules);
    findings.extend(test_layout_findings(&scope.project_root, &policy, &rules));
    findings.extend(source_test_mount_findings(scope, modules, &rules));
    findings.extend(test_leaf_bloat_findings(&scope.project_root, &rules));
    findings.extend(library_cargo_test_gate_findings(
        &reasoning_tree,
        scope,
        modules,
        &cargo_manifest,
        &rules,
    ));
    findings.extend(test_target_gate_findings(
        &scope.project_root,
        &cargo_test_targets,
        &rules,
    ));
    findings.extend(test_target_aggregate_findings(
        &scope.project_root,
        &cargo_test_targets,
        &rules,
    ));
    findings.extend(test_target_module_mount_findings(
        &scope.project_root,
        &cargo_test_targets,
        &policy,
        &rules,
    ));
    findings
}
