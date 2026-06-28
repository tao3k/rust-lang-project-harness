use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustScenarioBenchmarkDuration, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolationKind,
    assert_rule_fixture_scenario_benchmarks, validate_required_rust_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};
use tempfile::TempDir;

fn render_rust_scenario_benchmark_snapshot(receipt: &RustScenarioBenchmarkReceipt) -> String {
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
        format!(
            "reference_repositories: {}",
            receipt.scenario.reference_repositories.join(",")
        ),
        format!(
            "reference_patterns: {}",
            receipt.scenario.reference_patterns.join(" | ")
        ),
        format!("target_rationale: {}", receipt.benchmark.target_rationale),
        format!("inputs: {}", receipt.scenario.inputs),
        format!("expected: {}", receipt.scenario.expected),
    ];
    if let Some(comparison) = &receipt.benchmark.input_expected_comparison {
        lines.push(format!(
            "comparison_total: input={} expected={}",
            comparison.input_total, comparison.expected_total
        ));
        lines.push(format!(
            "comparison_memory_bytes: input={} expected={}",
            comparison.input_memory_bytes, comparison.expected_memory_bytes
        ));
        lines.push(format!(
            "comparison_interpretation: {}",
            comparison.interpretation
        ));
        lines.push(format!(
            "comparison_wall_time_delta: expected_vs_input={} speedup={}",
            relative_change(
                comparison.input_total.as_duration().as_nanos(),
                comparison.expected_total.as_duration().as_nanos()
            ),
            speedup(
                comparison.input_total.as_duration().as_nanos(),
                comparison.expected_total.as_duration().as_nanos()
            )
        ));
        lines.push(format!(
            "comparison_memory_delta: expected_vs_input={} reduction={}",
            relative_change(
                u128::from(comparison.input_memory_bytes.as_u64()),
                u128::from(comparison.expected_memory_bytes.as_u64())
            ),
            reduction(
                u128::from(comparison.input_memory_bytes.as_u64()),
                u128::from(comparison.expected_memory_bytes.as_u64())
            )
        ));
        if let Some(annotation) = &comparison.expected_not_faster_annotation {
            lines.push(format!(
                "comparison_expected_not_faster_annotation: {annotation}"
            ));
        }
    }
    lines.push(format!(
        "scenario_gate_timings: {}",
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

fn render_rust_scenario_benchmark_suite_snapshot(
    receipt: &RustScenarioBenchmarkSuiteReceipt,
) -> String {
    let mut lines = vec![
        format!("status: {}", receipt.status.as_str()),
        format!("requirements: {}", receipt.requirements.len()),
        format!("receipts: {}", receipt.receipts.len()),
        format!("policy_coverage: {}", receipt.policy_coverage.len()),
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
    lines.push("policy_coverage:".to_string());
    for coverage in &receipt.policy_coverage {
        lines.push(format!(
            "- {} {} {} {}",
            coverage.rule_id.as_str(),
            coverage.policy_id.as_str(),
            coverage.scenario_id.as_str(),
            display_suite_path(&receipt.root, &coverage.root)
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

fn display_suite_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn relative_change(input: u128, expected: u128) -> String {
    if input == 0 {
        return "n/a".to_string();
    }
    let change = ((expected as f64 - input as f64) / input as f64) * 100.0;
    format!("{change:+.2}%")
}

fn speedup(input: u128, expected: u128) -> String {
    if expected == 0 {
        return "n/a".to_string();
    }
    let speedup = input as f64 / expected as f64;
    format!("{speedup:.2}x")
}

fn reduction(input: u128, expected: u128) -> String {
    if input == 0 {
        return "n/a".to_string();
    }
    let reduction = ((input as f64 - expected as f64) / input as f64) * 100.0;
    format!("{reduction:.2}%")
}

#[test]
fn scenario_benchmark_control_flow_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/control_flow_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate control-flow scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_control_flow_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_data_structure_linear_membership_scan_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/data_structure_linear_membership_scan_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate data-structure linear membership scan scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-DS-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("linear membership scan scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes <= comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_data_structure_linear_membership_scan_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_process_command_probe_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/process_command_probe_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate process-command probe scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("process-command probe scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_process_command_probe_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_async_blocking_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_blocking_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async blocking boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-BLOCKING-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async blocking boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes <= comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_async_blocking_boundary_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_async_sync_lock_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_sync_lock_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async sync lock boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-SYNC-LOCK-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async sync lock boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_async_sync_lock_boundary_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_async_backpressure_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_backpressure_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async backpressure boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-BACKPRESSURE-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async backpressure boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_async_backpressure_boundary_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_async_select_cancellation_safety_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_select_cancellation_safety_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async select cancellation safety scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-CANCEL-SAFETY-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async select cancellation safety scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_async_select_cancellation_safety_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_async_timeout_cancellation_safety_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_timeout_cancellation_safety_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async timeout cancellation safety scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-CANCEL-SAFETY-002"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async timeout cancellation safety scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_async_timeout_cancellation_safety_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_rust_package_edition_2024_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/rust_package_edition_2024_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate Rust package edition 2024 scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("edition scenario should compare input and expected");
    assert!(
        comparison.expected_total <= comparison.input_total,
        "{comparison:?}"
    );

    insta::assert_snapshot!(
        "scenario_benchmark_rust_package_edition_2024_v1",
        render_rust_scenario_benchmark_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_suite_covers_all_required_current_scenarios() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate required scenario benchmark suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(receipt.receipts.len(), receipt.requirements.len());
    for rule_id in [
        "AGENT-R015",
        "AGENT-R016",
        "AGENT-R017",
        "AGENT-R025",
        "AGENT-R026",
        "AGENT-R029",
        "AGENT-R030",
        "AGENT-R031",
        "AGENT-R032",
        "AGENT-R033",
        "AGENT-R034",
        "RUST-AGENT-PROJECT-MANIFEST-023",
    ] {
        assert!(
            receipt
                .policy_coverage
                .iter()
                .any(|coverage| coverage.rule_id.as_str() == rule_id),
            "missing policy scenario coverage for {rule_id}: {receipt:?}"
        );
    }
    assert!(receipt.policy_coverage.iter().all(|coverage| {
        receipt
            .receipts
            .iter()
            .any(|scenario_receipt| scenario_receipt.root == coverage.root)
    }));
    assert!(receipt.receipts.iter().all(|receipt| {
        receipt.benchmark.observed_total <= receipt.benchmark.max_total
            && receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes
    }));

    insta::assert_snapshot!(
        "scenario_benchmark_required_suite",
        render_rust_scenario_benchmark_suite_snapshot(&receipt)
    );
}

#[test]
fn scenario_benchmark_hard_gate_accepts_current_required_suite() {
    assert_rule_fixture_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"));
}

#[test]
fn scenario_benchmark_numeric_gate_reports_speed_and_memory_failures() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_numeric_gate_reports_speed_and_memory_failures"
snapshot = "scenario_benchmark_numeric_gate_reports_speed_and_memory_failures"
target_total = "25ms"
max_total = "100ms"
observed_total = "140ms"
regression_budget = "20ms"
memory_budget_bytes = 1024
observed_memory_bytes = 2048
target_rationale = "The fixture should stay bounded."

[observed_timings]
parse = "120ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Memory
            && violation.field == "benchmark.observed_memory_bytes"
    }));
}

#[test]
fn scenario_benchmark_contract_accepts_libtest_insta_snapshot_entry() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "workspace_file_rejection_error_snapshot_and_perf"
snapshot = "workspace_file_rejection_error_snapshot_and_perf"
target_total = "750us"
max_total = "1.2ms"
observed_total = "900\u00b5s"
regression_budget = "250us"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Workspace argument validation is an in-process Rust API path."

[observed_timings]
workspace_metadata = "750\u00b5s"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert_eq!(receipt.benchmark.harness, "libtest");
    assert_eq!(
        receipt.benchmark.bench_entry(),
        "harness=libtest test=workspace_file_rejection_error_snapshot_and_perf snapshot=workspace_file_rejection_error_snapshot_and_perf"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
}

#[test]
fn scenario_benchmark_contract_requires_agent_visible_metadata() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario_with_policy_ids(temp.path(), "[]");
    write_benchmark(
        temp.path(),
        r#"
harness = ""
test = ""
target_total = "120ms"
max_total = "100ms"
observed_total = "90ms"
regression_budget = "0ms"
memory_budget_bytes = 0
observed_memory_bytes = 0
target_rationale = ""
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "scenario.policy_ids"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.observed_timings"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.target_total"
    }));
}

#[test]
fn scenario_benchmark_comparison_allows_expected_to_be_slower_than_input() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_comparison_allows_expected_to_be_slower_than_input"
snapshot = "scenario_benchmark_comparison_allows_expected_to_be_slower_than_input"
target_total = "25ms"
max_total = "100ms"
observed_total = "30ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "The expected fixture may trade a small runtime cost for clearer safety boundaries."

[input_expected_comparison]
input_total = "9ms"
expected_total = "12ms"
input_memory_bytes = 1048576
expected_memory_bytes = 2097152
interpretation = "The input fixture is faster here, but the expected fixture documents the safe owner boundary."
expected_not_faster_annotation = "Expected is intentionally slower here because the scenario is validating annotation behavior."

[observed_timings]
fixture = "30ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .expect("comparison is part of the contract");
    assert!(comparison.expected_total > comparison.input_total);
    assert!(comparison.expected_not_faster_annotation.is_some());
}

#[test]
fn scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster"
snapshot = "scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster"
target_total = "25ms"
max_total = "100ms"
observed_total = "30ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "The expected fixture must annotate when it does not beat input."

[input_expected_comparison]
input_total = "9ms"
expected_total = "12ms"
input_memory_bytes = 1048576
expected_memory_bytes = 2097152
interpretation = "This is incomplete because the slower expected fixture has no annotation."

[observed_timings]
fixture = "30ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field
                == "benchmark.input_expected_comparison.expected_not_faster_annotation"
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_self_referential_gate_command() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "orgize_rule_fixtures_have_scenario_benchmarks"
target_total = "25ms"
max_total = "100ms"
observed_total = "25ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Small rule fixture should stay bounded."

[observed_timings]
fixture = "25ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.entry"
            && violation
                .message
                .contains("focused Rust test or bench case")
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_second_scale_hard_gate() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "criterion"
bench = "asp_search_deps"
case = "tokio"
target_total = "250ms"
max_total = "5s"
observed_total = "240ms"
regression_budget = "100ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Dependency seed should stay inside the millisecond gate."

[observed_timings]
asp_search_deps = "240ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.max_total"
            && violation.message.contains("hard gate")
    }));
}

#[test]
fn scenario_benchmark_suite_reports_missing_required_benchmark() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("missing_benchmark");
    fs::create_dir_all(&scenario_root).expect("create scenario root");
    write_scenario(&scenario_root);

    let receipt = validate_required_rust_scenario_benchmarks(temp.path()).expect("validate suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert_eq!(receipt.requirements.len(), 1);
    assert!(receipt.receipts.is_empty());
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "tests/unit/scenarios/missing_benchmark/benchmark.toml"
    }));

    let panic = std::panic::catch_unwind(|| {
        assert_rule_fixture_scenario_benchmarks(temp.path());
    })
    .expect_err("hard gate should panic for missing benchmark");
    let message = panic_message(panic);
    assert!(message.contains("scenario benchmark hard gate failed"));
    assert!(message.contains("preferred fix: add benchmark.toml"));
    assert!(message.contains("target_total = \"25ms\""));
    assert!(message.contains("memory_budget_bytes = 8388608"));
    assert!(!message.contains("advisory mode ="));
    assert!(!message.contains("expires ="));
}

#[test]
fn scenario_benchmark_hard_gate_panics_with_repair_template() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("missing_benchmark");
    fs::create_dir_all(&scenario_root).expect("create scenario root");
    write_scenario(&scenario_root);

    let panic = std::panic::catch_unwind(|| {
        assert_rule_fixture_scenario_benchmarks(temp.path());
    })
    .expect_err("hard gate should panic for missing benchmark");
    let message = panic_message(panic);

    assert!(message.contains("scenario benchmark hard gate failed"));
    assert!(message.contains("tests/unit/scenarios/missing_benchmark/benchmark.toml"));
    assert!(message.contains("preferred fix: add benchmark.toml"));
    assert!(message.contains("harness = \"libtest\""));
    assert!(message.contains("test = \"<focused-libtest-case>\""));
    assert!(message.contains("snapshot = \"<insta-snapshot-name>\""));
    assert!(message.contains("[input_expected_comparison]"));
    assert!(!message.contains("advisory mode ="));
    assert!(!message.contains("expires ="));
}

#[test]
fn scenario_benchmark_suite_reports_ast_patch_speed_failure() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios")
        .join("slow_apply");
    fs::create_dir_all(&scenario_root).expect("create ast patch scenario root");
    fs::write(
        scenario_root.join("scenario.json"),
        r#"
{
  "mode": "apply",
  "expectedStatus": "applied",
  "expectedCapability": "provider-ast-apply",
  "expectedMutationAvailable": true,
  "expectedOperation": "replace_item"
}
"#,
    )
    .expect("write ast patch scenario");
    write_benchmark(
        &scenario_root,
        r#"
harness = "libtest"
test = "ast_patch_scenarios::slow_apply"
snapshot = "ast_patch_scenarios::slow_apply"
target_total = "25ms"
max_total = "100ms"
observed_total = "140ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "AST patch scenario should stay bounded."

[observed_timings]
manifest = "5ms"
apply = "120ms"
"#,
    );

    let receipt = validate_required_rust_scenario_benchmarks(temp.path()).expect("validate suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert_eq!(receipt.requirements.len(), 1);
    assert_eq!(receipt.receipts.len(), 1);
    assert!(receipt.receipts[0].violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total"
    }));
}

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join(name)
}

fn write_scenario(root: &Path) {
    write_scenario_with_policy_ids(root, r#"["RUST-AGENT-CFG-001"]"#);
}

fn write_scenario_with_policy_ids(root: &Path, policy_ids: &str) {
    fs::write(
        root.join("scenario.toml"),
        format!(
            r#"
id = "contract-test"
title = "Contract test"
policy_ids = {policy_ids}
agent_goal = "Keep the scenario understandable."
reference_repositories = ["rust-lang/rust"]
reference_patterns = ["Test fixtures still name the source of the contract pattern"]
inputs = "inputs"
expected = "expected"
"#
        ),
    )
    .expect("write scenario");
}

fn write_benchmark(root: &Path, text: &str) {
    fs::write(root.join("benchmark.toml"), text).expect("write benchmark");
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    "<non-string panic>".to_string()
}
