//! Project-policy quality checks for waiver and wrapper surfaces.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::{RustHarnessConfig, RustHarnessFinding, RustHarnessRule};

use super::support::display_project_path;
use super::{RUST_PROJ_R017, RUST_PROJ_R018, RUST_PROJ_R019};

pub(super) fn quality_findings(
    project_root: &Path,
    config: &RustHarnessConfig,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    findings.extend(weak_advice_explanation_findings(
        project_root,
        config,
        rules,
    ));
    findings.extend(fake_identity_fallback_findings(
        project_root,
        modules,
        rules,
    ));
    findings.extend(redundant_workspace_wrapper_findings(
        project_root,
        modules,
        rules,
    ));
    findings
}

fn weak_advice_explanation_findings(
    project_root: &Path,
    config: &RustHarnessConfig,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    [
        (
            "agent_advice_allow_explanation",
            config.agent_advice_allow_explanation.as_deref(),
        ),
        (
            "cargo_check_advice_allow_explanation",
            config.cargo_check_advice_allow_explanation.as_deref(),
        ),
        (
            "cargo_test_advice_allow_explanation",
            config.cargo_test_advice_allow_explanation.as_deref(),
        ),
    ]
    .into_iter()
    .filter_map(|(field_name, explanation)| {
        let explanation = explanation?;
        if advice_explanation_is_structured(explanation) {
            return None;
        }
        let rule = &rules[RUST_PROJ_R017];
        Some(RustHarnessFinding::from_rule(
            rule,
            format!(
                "{field_name} is configured without a structured allowance contract."
            ),
            file_location(build_script_or_manifest(project_root)),
            None,
            "rewrite the explanation with scope=..., owner=..., finding_category=..., why_safe_now=..., and cleanup_trigger=...",
        ))
    })
    .collect()
}

fn advice_explanation_is_structured(explanation: &str) -> bool {
    let lower = explanation.to_ascii_lowercase();
    [
        "scope=",
        "owner=",
        "finding_category=",
        "why_safe_now=",
        "cleanup_trigger=",
    ]
    .into_iter()
    .all(|marker| lower.contains(marker))
}

fn fake_identity_fallback_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_PROJ_R018];
    modules
        .iter()
        .filter(|module| module.source.contains("CARGO_PKG_NAME"))
        .filter_map(|module| {
            let line = module
                .source
                .lines()
                .position(line_has_fake_identity_fallback)?;
            let line = line + 1;
            Some(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} synthesizes a fake Cargo package identity fallback.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "fail closed when CARGO_PKG_NAME is missing or empty, or pass the crate label explicitly from the caller",
            ))
        })
        .collect()
}

fn line_has_fake_identity_fallback(line: &str) -> bool {
    [
        "\"unknown\"",
        "\"unknown-cargo-package\"",
        "\"unknown-crate\"",
        "\"unknown-package\"",
        "\"default-crate\"",
        "\"todo\"",
    ]
    .into_iter()
    .any(|sentinel| line.contains(sentinel))
}

fn redundant_workspace_wrapper_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_PROJ_R019];
    modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .top_level_items
                .iter()
                .filter_map(move |item| {
                    let function_name = item.function_name.as_deref()?;
                    if !item.is_public
                        || !is_redundant_member_build_gate_alias(function_name)
                        || !item_range_contains_harness_alias_target(module, item.line, item.end_line)
                    {
                        return None;
                    }
                    Some(RustHarnessFinding::from_rule(
                        rule,
                        format!(
                            "{} exposes redundant public build-gate alias `{function_name}`.",
                            display_project_path(project_root, &module.report.path)
                        ),
                        path_line_location(&module.report.path, item.line),
                        source_line(&module.source, item.line),
                        "remove the alias, or add deprecation metadata plus a bounded migration deadline",
                    ))
                })
        })
        .collect()
}

fn is_redundant_member_build_gate_alias(function_name: &str) -> bool {
    function_name.starts_with("assert_member_")
        && function_name.contains("build_gate")
        && function_name.contains("_from_env")
        && !function_name.contains("harness")
}

fn item_range_contains_harness_alias_target(
    module: &ParsedRustModule,
    start_line: usize,
    end_line: usize,
) -> bool {
    module
        .source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .any(|line| line.contains("assert_member_harness_build_gate_from_env"))
}

fn build_script_or_manifest(project_root: &Path) -> std::path::PathBuf {
    let build_script = project_root.join("build.rs");
    if build_script.exists() {
        build_script
    } else {
        project_root.join("Cargo.toml")
    }
}
