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
fn scenario_benchmark_suite_covers_all_required_current_scenarios() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate required scenario benchmark suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(receipt.requirements.len(), 13, "{receipt:?}");
    assert_eq!(receipt.receipts.len(), receipt.requirements.len());
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

    let rendered = render_rust_scenario_benchmark_gate_failure(&receipt);
    assert!(rendered.contains("scenario benchmark hard gate failed"));
    assert!(rendered.contains("preferred fix: add benchmark.toml"));
    assert!(rendered.contains("target_total = \"25ms\""));
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
    assert!(message.contains("harness = \"libtest\""));
    assert!(message.contains("test = \"<focused-libtest-case>\""));
    assert!(message.contains("snapshot = \"<insta-snapshot-name>\""));
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
