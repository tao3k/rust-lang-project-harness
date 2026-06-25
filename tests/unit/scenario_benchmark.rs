use std::fs;
use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkViolationKind,
    assert_rule_fixture_scenario_benchmarks, render_rust_scenario_benchmark_gate_failure,
    render_rust_scenario_benchmark_snapshot, render_rust_scenario_benchmark_suite_snapshot,
    validate_required_rust_scenario_benchmarks, validate_rust_scenario_benchmark,
};
use tempfile::TempDir;

#[test]
fn scenario_benchmark_control_flow_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/control_flow_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate control-flow scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total_ms <= receipt.benchmark.max_total_ms,
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
fn scenario_benchmark_suite_covers_all_required_current_scenarios() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate required scenario benchmark suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(receipt.requirements.len(), 13, "{receipt:?}");
    assert_eq!(receipt.receipts.len(), receipt.requirements.len());
    assert!(receipt.receipts.iter().all(|receipt| {
        receipt.benchmark.observed_total_ms <= receipt.benchmark.max_total_ms
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
bench_command = "cargo test slow"
target_total_ms = 25
max_total_ms = 100
observed_total_ms = 140
regression_budget_ms = 20
memory_budget_bytes = 1024
observed_memory_bytes = 2048
target_rationale = "The fixture should stay bounded."

[observed_timings]
parse_ms = 120
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total_ms"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Memory
            && violation.field == "benchmark.observed_memory_bytes"
    }));
}

#[test]
fn scenario_benchmark_contract_requires_agent_visible_metadata() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario_with_policy_ids(temp.path(), "[]");
    write_benchmark(
        temp.path(),
        r#"
bench_command = ""
target_total_ms = 120
max_total_ms = 100
observed_total_ms = 90
regression_budget_ms = 0
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
            && violation.field == "benchmark.target_total_ms"
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_self_referential_gate_command() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
bench_command = "cargo test --test integration_test orgize_rule_fixtures_have_scenario_benchmarks"
target_total_ms = 25
max_total_ms = 100
observed_total_ms = 25
regression_budget_ms = 20
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Small rule fixture should stay bounded."

[observed_timings]
fixture_ms = 25
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.bench_command"
            && violation
                .message
                .contains("focused scenario benchmark test")
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_second_scale_hard_gate() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
bench_command = "target/debug/asp rust search deps tokio --workspace . --view hits"
target_total_ms = 250
max_total_ms = 5000
observed_total_ms = 240
regression_budget_ms = 100
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Dependency seed should stay inside the millisecond gate."

[observed_timings]
asp_search_deps_ms = 240
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.max_total_ms"
            && violation.message.contains("millisecond hard gate")
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

    let rendered = render_rust_scenario_benchmark_gate_failure(&receipt);
    assert!(rendered.contains("scenario benchmark hard gate failed"));
    assert!(rendered.contains("preferred fix: add benchmark.toml"));
    assert!(rendered.contains("target_total_ms = 25"));
    assert!(rendered.contains("memory_budget_bytes = 8388608"));
    assert!(!rendered.contains("advisory mode ="));
    assert!(!rendered.contains("expires ="));
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
    assert!(message.contains("bench_command = \"cargo test <focused-test>\""));
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
bench_command = "cargo test slow ast patch"
target_total_ms = 25
max_total_ms = 100
observed_total_ms = 140
regression_budget_ms = 20
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "AST patch scenario should stay bounded."

[observed_timings]
manifest_ms = 5
apply_ms = 120
"#,
    );

    let receipt = validate_required_rust_scenario_benchmarks(temp.path()).expect("validate suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert_eq!(receipt.requirements.len(), 1);
    assert_eq!(receipt.receipts.len(), 1);
    assert!(receipt.receipts[0].violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total_ms"
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
    write_scenario_with_policy_ids(root, r#"["RUST-CFG-R001"]"#);
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
