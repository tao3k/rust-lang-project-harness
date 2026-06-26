//! Scenario benchmark receipt rendering.

use std::collections::BTreeMap;
use std::path::Path;

use super::contract::RustScenarioBenchmarkDuration;
use super::contract_gate::default_benchmark_toml_template;
use super::types::{RustScenarioBenchmarkReceipt, RustScenarioBenchmarkSuiteReceipt};

/// Render an `insta` snapshot for a scenario benchmark receipt.
///
/// Observed measurements are normalized to keep snapshots stable while the
/// numeric gate still checks real values from `benchmark.toml`.
pub fn render_rust_scenario_benchmark_snapshot(receipt: &RustScenarioBenchmarkReceipt) -> String {
    let mut lines = vec![
        format!("scenario: {}", receipt.scenario.id),
        format!("title: {}", receipt.scenario.title),
        format!("status: {}", receipt.status.as_str()),
        format!("policies: {}", receipt.scenario.policy_ids.join(",")),
        format!("bench_entry: {}", receipt.benchmark.bench_entry()),
        "observed_total: <measured>".to_string(),
        format!("target_total: {}", receipt.benchmark.target_total),
        format!("max_total: {}", receipt.benchmark.max_total),
        "observed_memory_bytes: <measured>".to_string(),
        format!(
            "memory_budget_bytes: {}",
            receipt.benchmark.memory_budget_bytes
        ),
        format!("regression_budget: {}", receipt.benchmark.regression_budget),
        format!("agent_goal: {}", receipt.scenario.agent_goal),
        format!("target_rationale: {}", receipt.benchmark.target_rationale),
        format!("inputs: {}", receipt.scenario.inputs),
        format!("expected: {}", receipt.scenario.expected),
    ];
    lines.push(format!(
        "timings: {}",
        normalized_timings(&receipt.benchmark.observed_timings)
    ));
    if receipt.violations.is_empty() {
        lines.push("violations: -".to_string());
    } else {
        lines.push("violations:".to_string());
        for violation in &receipt.violations {
            lines.push(format!(
                "- {}:{}: {}",
                violation.kind.as_str(),
                violation.field,
                violation.message
            ));
        }
    }
    lines.join("\n")
}

/// Render an `insta` snapshot for the full scenario benchmark suite.
pub fn render_rust_scenario_benchmark_suite_snapshot(
    receipt: &RustScenarioBenchmarkSuiteReceipt,
) -> String {
    let mut lines = vec![
        format!("status: {}", receipt.status.as_str()),
        format!("requirements: {}", receipt.requirements.len()),
        format!("receipts: {}", receipt.receipts.len()),
    ];
    lines.push("required:".to_string());
    for requirement in &receipt.requirements {
        lines.push(format!(
            "- {} {}",
            requirement.manifest_kind.as_str(),
            display_suite_path(&receipt.root, &requirement.root)
        ));
    }
    lines.push("scenario_status:".to_string());
    for scenario_receipt in &receipt.receipts {
        lines.push(format!(
            "- {} {}",
            scenario_receipt.status.as_str(),
            display_suite_path(&receipt.root, &scenario_receipt.root)
        ));
    }
    if receipt.violations.is_empty() {
        lines.push("violations: -".to_string());
    } else {
        lines.push("violations:".to_string());
        for violation in &receipt.violations {
            lines.push(format!(
                "- {}:{}: {}",
                violation.kind.as_str(),
                violation.field,
                violation.message
            ));
        }
    }
    lines.join("\n")
}

/// Render the hard-gate failure message shown to downstream unit tests.
pub fn render_rust_scenario_benchmark_gate_failure(
    receipt: &RustScenarioBenchmarkSuiteReceipt,
) -> String {
    let mut lines = vec![
        "scenario benchmark hard gate failed".to_string(),
        "preferred fix: add benchmark.toml next to the scenario fixture".to_string(),
        "do not add advisory mode, fixture-local opt-out, or expires".to_string(),
        format!("status: {}", receipt.status.as_str()),
    ];
    for violation in &receipt.violations {
        lines.push(format!(
            "- {}:{}: {}",
            violation.kind.as_str(),
            violation.field,
            violation.message
        ));
        if violation.field.ends_with("benchmark.toml") {
            lines.push(format!("create: {}", violation.field));
            lines.push(default_benchmark_toml_template());
        }
    }
    for scenario_receipt in &receipt.receipts {
        for violation in &scenario_receipt.violations {
            lines.push(format!(
                "- {}:{}:{}: {}",
                violation.kind.as_str(),
                display_suite_path(&receipt.root, &scenario_receipt.root),
                violation.field,
                violation.message
            ));
        }
    }
    lines.join("\n")
}

pub(super) fn display_suite_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalized_timings(timings: &BTreeMap<String, RustScenarioBenchmarkDuration>) -> String {
    if timings.is_empty() {
        return "-".to_string();
    }
    timings
        .keys()
        .map(|key| format!("{key}=<measured>"))
        .collect::<Vec<_>>()
        .join(",")
}
