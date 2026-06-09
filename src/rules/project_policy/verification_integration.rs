//! Verification policy integration checks backed by Cargo manifest facts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::parser::{
    CargoBenchTargetFacts, CargoDependencyFacts, CargoDependencyKind, CargoManifestFacts,
    ParsedRustModule, RustReasoningTreeFacts, file_location, parse_cargo_dependency_facts,
    path_line_location, source_line,
};
use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};
use crate::{RustHarnessConfig, RustHarnessFinding, RustHarnessRule};

use super::build_gate::{module_default_build_gate_call_lines, root_build_script_module};
use super::support::display_project_path;
use super::{RUST_PROJ_R009, RUST_PROJ_R010, RUST_PROJ_R011, RUST_PROJ_R015, RUST_PROJ_R016};

const SOURCE_CARGO_TEST_GATE_MACROS: &[&str] = &[
    "rust_project_harness_gate",
    "rust_project_harness_cargo_test_gate",
];

pub(super) fn verification_integration_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    config: &RustHarnessConfig,
    modules: &[ParsedRustModule],
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(legacy_source_cargo_test_gate_findings(
        project_root,
        reasoning_tree,
        modules,
        cargo_manifest,
        rules,
    ));
    findings.extend(empty_cargo_test_gate_config_findings(
        project_root,
        reasoning_tree,
        modules,
        rules,
    ));
    findings.extend(advice_allow_explanation_findings(
        project_root,
        reasoning_tree,
        config,
        modules,
        rules,
    ));
    findings.extend(empty_build_gate_config_findings(
        project_root,
        modules,
        rules,
    ));
    let performance_adapters = active_rust_native_performance_adapters(project_root, config);
    if performance_adapters.is_empty() {
        return findings;
    }
    if has_rust_native_performance_bench(project_root, cargo_manifest, &performance_adapters) {
        return findings;
    }

    let rule = &rules[RUST_PROJ_R010];
    findings.push(RustHarnessFinding::from_rule(
        rule,
        format!(
            "{} configures a Rust-native performance verification skill, but Cargo.toml does not expose a runnable Criterion, Divan, or iai-callgrind harness=false [[bench]] target.",
            display_project_path(project_root, &project_root.join("Cargo.toml"))
        ),
        file_location(project_root.join("Cargo.toml")),
        None,
        "add a Criterion, Divan, or iai-callgrind [[bench]] target, declare the matching benchmark framework dependency, and keep the verification contract command pointed at it",
    ));
    findings
}

fn legacy_source_cargo_test_gate_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !cargo_manifest.references_harness {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R009];
    source_modules(reasoning_tree, modules)
        .filter_map(|module| {
            let invocation = module
                .syntax_facts
                .macro_invocations
                .iter()
                .find(|invocation| {
                    SOURCE_CARGO_TEST_GATE_MACROS.contains(&invocation.terminal_name.as_str())
                })?;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} mounts a legacy source cargo-test harness gate.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, invocation.line),
                source_line(&module.source, invocation.line),
                "move parser-native harness policy to [build-dependencies] plus root build.rs using assert_rust_project_harness_cargo_check_clean_from_env_with_config(...), then remove this cargo-test source gate",
            ))
        })
        .collect()
}

fn empty_build_gate_config_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let Some(module) = root_build_script_module(project_root, modules) else {
        return Vec::new();
    };
    let rule = &rules[RUST_PROJ_R011];
    module_default_build_gate_call_lines(module)
        .map(|line| {
            RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} mounts the build-time harness gate without explicit verification config.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "use assert_rust_project_harness_cargo_check_clean_from_env_with_config(...) and declare verification profile hints, explicit suppressions, or skill bindings",
            )
        })
        .collect()
}

fn empty_cargo_test_gate_config_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_PROJ_R016];
    source_modules(reasoning_tree, modules)
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

fn advice_allow_explanation_findings(
    project_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    config: &RustHarnessConfig,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if config_allows_cargo_test_advice(config) {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R015];
    source_modules(reasoning_tree, modules)
        .filter_map(|module| {
            let invocation = module.syntax_facts.macro_invocations.iter().find(|invocation| {
                invocation.terminal_name == "rust_project_harness_cargo_test_gate"
                    && invocation
                        .argument_top_level_idents
                        .iter()
                        .any(|ident| ident == "advice")
                    && invocation
                        .argument_top_level_idents
                        .iter()
                        .any(|ident| ident == "allow")
            })?;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} mounts the cargo-test harness gate with advice allowance but no explicit allow explanation.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, invocation.line),
                source_line(&module.source, invocation.line),
                "use with_cargo_test_advice_allow_explanation(...) to explain why advisory findings may pass this legacy cargo-test gate",
            ))
        })
        .collect()
}

fn config_allows_cargo_test_advice(config: &RustHarnessConfig) -> bool {
    has_explanation(config.cargo_test_advice_allow_explanation.as_deref())
        || has_explanation(config.agent_advice_allow_explanation.as_deref())
}

fn has_explanation(explanation: Option<&str>) -> bool {
    explanation.is_some_and(|explanation| !explanation.trim().is_empty())
}

fn source_modules<'a>(
    reasoning_tree: &'a RustReasoningTreeFacts,
    modules: &'a [ParsedRustModule],
) -> impl Iterator<Item = &'a ParsedRustModule> {
    modules.iter().filter(|module| {
        reasoning_tree
            .module(&module.report.path)
            .is_some_and(|facts| facts.is_source_module)
    })
}

fn active_rust_native_performance_adapters(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> BTreeSet<String> {
    plan_rust_project_verification_with_config(project_root, config)
        .map(|plan| rust_native_performance_adapters(&plan))
        .unwrap_or_default()
}

fn rust_native_performance_adapters(plan: &RustVerificationPlan) -> BTreeSet<String> {
    plan.active_tasks()
        .into_iter()
        .filter(|task| task.kind == RustVerificationTaskKind::Performance)
        .filter_map(|task| {
            task.skill_binding
                .as_ref()
                .and_then(|binding| binding.adapter.as_deref())
        })
        .filter(|adapter| is_rust_native_performance_adapter(adapter))
        .map(ToOwned::to_owned)
        .collect()
}

fn is_rust_native_performance_adapter(adapter: &str) -> bool {
    matches!(
        adapter,
        "criterion" | "divan" | "iai-callgrind" | "iai_callgrind"
    )
}

fn has_rust_native_performance_bench(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    adapters: &BTreeSet<String>,
) -> bool {
    let dependencies = parse_cargo_dependency_facts(project_root);
    cargo_manifest
        .bench_targets
        .iter()
        .any(|target| target_matches_rust_native_performance(target, adapters, &dependencies))
}

fn target_matches_rust_native_performance(
    target: &CargoBenchTargetFacts,
    adapters: &BTreeSet<String>,
    dependencies: &[CargoDependencyFacts],
) -> bool {
    !target.harness
        && target.path.exists()
        && adapters.iter().any(|adapter| {
            manifest_has_adapter_dependency(dependencies, adapter)
                && bench_target_uses_adapter(target, adapter)
        })
}

fn manifest_has_adapter_dependency(dependencies: &[CargoDependencyFacts], adapter: &str) -> bool {
    let package = adapter_package_name(adapter);
    dependencies.iter().any(|dependency| {
        matches!(
            dependency.kind,
            CargoDependencyKind::Normal | CargoDependencyKind::Dev
        ) && dependency.package_name == package
    })
}

fn adapter_package_name(adapter: &str) -> &'static str {
    match adapter {
        "criterion" => "criterion",
        "divan" => "divan",
        "iai-callgrind" | "iai_callgrind" => "iai-callgrind",
        _ => "",
    }
}

fn bench_target_uses_adapter(target: &CargoBenchTargetFacts, adapter: &str) -> bool {
    let Ok(source) = fs::read_to_string(&target.path) else {
        return false;
    };
    match adapter {
        "criterion" => {
            source_uses_criterion_macros(&source) || source_uses_manual_criterion_main(&source)
        }
        "divan" => source.contains("divan::main") || source.contains("#[divan::bench]"),
        "iai-callgrind" | "iai_callgrind" => {
            source.contains("iai_callgrind::")
                || source.contains("library_benchmark_group!")
                || source.contains("main!(")
        }
        _ => false,
    }
}

fn source_uses_criterion_macros(source: &str) -> bool {
    source.contains("criterion_group!") && source.contains("criterion_main!")
}

fn source_uses_manual_criterion_main(source: &str) -> bool {
    source.contains("Criterion::")
        && source.contains(".final_summary()")
        && (source.contains(".bench_function(") || source.contains(".benchmark_group("))
}
