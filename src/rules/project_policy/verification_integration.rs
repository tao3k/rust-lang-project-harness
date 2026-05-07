//! Verification policy integration checks backed by Cargo manifest facts.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{
    CargoManifestFacts, ParsedRustModule, RustReasoningTreeFacts, file_location,
    path_line_location, source_line,
};
use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};
use crate::{RustHarnessConfig, RustHarnessFinding, RustHarnessRule};

use super::support::display_project_path;
use super::{RUST_PROJ_R010, RUST_PROJ_R011};

pub(super) fn verification_integration_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    config: &RustHarnessConfig,
    modules: &[ParsedRustModule],
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(empty_verification_config_gate_findings(
        project_root,
        reasoning_tree,
        modules,
        rules,
    ));
    if !configured_rust_native_performance_is_active(project_root, config) {
        return findings;
    }
    if cargo_manifest
        .bench_targets
        .iter()
        .any(|target| !target.harness && target.path.exists())
    {
        return findings;
    }

    let rule = &rules[RUST_PROJ_R010];
    findings.push(RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} configures a Rust-native performance verification skill, but Cargo.toml does not expose a runnable harness=false [[bench]] target.",
            display_project_path(project_root, &project_root.join("Cargo.toml"))
        ),
        file_location(project_root.join("Cargo.toml")),
        None,
        "add a Criterion, Divan, or iai-callgrind [[bench]] target and keep the verification contract command pointed at it",
    ));
    findings
}

fn empty_verification_config_gate_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_PROJ_R011];
    modules
        .iter()
        .filter(|module| {
            reasoning_tree
                .module(&module.report.path)
                .is_some_and(|facts| facts.is_source_module)
        })
        .filter_map(|module| {
            let invocation = module
                .syntax_facts
                .macro_invocations
                .iter()
                .find(|invocation| {
                    invocation.terminal_name == "rust_project_harness_cargo_test_gate"
                        && !invocation
                            .argument_top_level_idents
                            .iter()
                            .any(|ident| ident == "config")
                })?;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} mounts the cargo-test harness gate without explicit verification config.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, invocation.line),
                source_line(&module.source, invocation.line),
                "use rust_project_harness_cargo_test_gate!(config = { ... }) and declare verification profile hints, explicit suppressions, or skill bindings",
            ))
        })
        .collect()
}

fn configured_rust_native_performance_is_active(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> bool {
    plan_rust_project_verification_with_config(project_root, config)
        .is_ok_and(|plan| plan_contains_active_rust_native_performance_task(&plan))
}

fn plan_contains_active_rust_native_performance_task(plan: &RustVerificationPlan) -> bool {
    plan.active_tasks().into_iter().any(|task| {
        task.kind == RustVerificationTaskKind::Performance
            && task
                .skill_binding
                .as_ref()
                .and_then(|binding| binding.adapter.as_deref())
                .is_some_and(is_rust_native_performance_adapter)
    })
}

fn is_rust_native_performance_adapter(adapter: &str) -> bool {
    matches!(
        adapter,
        "criterion" | "divan" | "iai-callgrind" | "iai_callgrind"
    )
}
