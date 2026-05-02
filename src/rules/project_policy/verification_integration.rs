//! Verification policy integration checks backed by Cargo manifest facts.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{CargoManifestFacts, file_location};
use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};
use crate::{RustHarnessConfig, RustHarnessFinding, RustHarnessRule};

use super::RUST_PROJ_R010;
use super::support::display_project_path;

pub(super) fn verification_integration_findings(
    project_root: &Path,
    config: &RustHarnessConfig,
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !configured_rust_native_performance_is_active(project_root, config) {
        return Vec::new();
    }
    if cargo_manifest
        .bench_targets
        .iter()
        .any(|target| !target.harness && target.path.exists())
    {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R010];
    vec![RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} configures a Rust-native performance verification skill, but Cargo.toml does not expose a runnable harness=false [[bench]] target.",
            display_project_path(project_root, &project_root.join("Cargo.toml"))
        ),
        file_location(project_root.join("Cargo.toml")),
        None,
        "add a Criterion, Divan, or iai-callgrind [[bench]] target and keep the verification contract command pointed at it",
    )]
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
