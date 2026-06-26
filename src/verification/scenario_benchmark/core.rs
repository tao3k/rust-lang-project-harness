//! Scenario benchmark contracts for Rust harness fixtures.

use std::fs;
use std::path::Path;
use std::time::Duration;

use super::contract::{RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration};
use super::contract_gate::benchmark_entry_targets_contract_gate;
use super::discovery::discover_required_rust_scenario_benchmarks;
use super::render::{display_suite_path, render_rust_scenario_benchmark_gate_failure};
use super::types::{
    RustScenarioBenchmarkError, RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkReceipt,
    RustScenarioBenchmarkRequirement, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
};
use serde::Deserialize;

const RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL: RustScenarioBenchmarkDuration =
    RustScenarioBenchmarkDuration(Duration::from_millis(500));

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RustAstPatchScenarioManifest {
    mode: String,
    expected_status: String,
    expected_capability: String,
    #[serde(default)]
    expected_operation: Option<String>,
}

/// Validate one scenario root containing `scenario.toml` and `benchmark.toml`.
pub fn validate_rust_scenario_benchmark(
    root: impl AsRef<Path>,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    let root = root.as_ref();
    let scenario_path = root.join("scenario.toml");
    let benchmark_path = root.join("benchmark.toml");
    let scenario: RustScenarioMetadata = read_toml(&scenario_path)?;
    let benchmark: RustScenarioBenchmarkContract = read_toml(&benchmark_path)?;
    let violations = scenario_benchmark_violations(&scenario, &benchmark);
    let status = scenario_benchmark_status(&violations);
    Ok(RustScenarioBenchmarkReceipt {
        root: root.to_path_buf(),
        scenario,
        benchmark,
        status,
        violations,
    })
}

/// Validate every required Rust harness scenario benchmark contract.
pub fn validate_required_rust_scenario_benchmarks(
    crate_root: impl AsRef<Path>,
) -> Result<RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkError> {
    let crate_root = crate_root.as_ref();
    let requirements = discover_required_rust_scenario_benchmarks(crate_root)?;
    let mut receipts = Vec::new();
    let mut violations = Vec::new();
    for requirement in &requirements {
        let benchmark_path = requirement.root.join("benchmark.toml");
        if !benchmark_path.exists() {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Contract,
                field: display_suite_path(crate_root, &benchmark_path),
                message: "required benchmark.toml is missing".to_string(),
            });
            continue;
        }
        receipts.push(validate_rust_scenario_benchmark_requirement(requirement)?);
    }
    let status = scenario_benchmark_suite_status(&receipts, &violations);
    Ok(RustScenarioBenchmarkSuiteReceipt {
        root: crate_root.to_path_buf(),
        requirements,
        receipts,
        violations,
        status,
    })
}

/// Assert that every discovered rule or scenario fixture has a benchmark contract.
///
/// This is the downstream unit-test entrypoint. It intentionally fails hard and
/// renders a repair template instead of returning an advisory warning.
pub fn assert_rule_fixture_scenario_benchmarks(crate_root: impl AsRef<Path>) {
    let crate_root = crate_root.as_ref();
    let receipt = validate_required_rust_scenario_benchmarks(crate_root).unwrap_or_else(|error| {
        panic!(
            "scenario benchmark hard gate could not read {}: {error}",
            crate_root.display()
        )
    });
    if receipt.status != RustScenarioBenchmarkStatus::Pass {
        panic!("{}", render_rust_scenario_benchmark_gate_failure(&receipt));
    }
}

fn validate_rust_scenario_benchmark_requirement(
    requirement: &RustScenarioBenchmarkRequirement,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    match requirement.manifest_kind {
        RustScenarioBenchmarkManifestKind::ScenarioToml => {
            validate_rust_scenario_benchmark(&requirement.root)
        }
        RustScenarioBenchmarkManifestKind::AstPatchScenarioJson => {
            validate_ast_patch_scenario_benchmark(&requirement.root)
        }
    }
}

fn validate_ast_patch_scenario_benchmark(
    root: &Path,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    let manifest: RustAstPatchScenarioManifest = read_json(&root.join("scenario.json"))?;
    let benchmark: RustScenarioBenchmarkContract = read_toml(&root.join("benchmark.toml"))?;
    let scenario = RustScenarioMetadata {
        id: root
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ast-patch-scenario".to_string()),
        title: ast_patch_scenario_title(root),
        policy_ids: vec!["RUST-AST-PATCH-SCENARIO".to_string()],
        agent_goal: format!(
            "Keep AST patch {mode} scenario bounded for {status}/{capability}.",
            mode = manifest.mode,
            status = manifest.expected_status,
            capability = manifest.expected_capability
        ),
        inputs: "input".to_string(),
        expected: "expected".to_string(),
    };
    let mut violations = scenario_benchmark_violations(&scenario, &benchmark);
    require_non_empty(&mut violations, "scenario.mode", &manifest.mode);
    require_non_empty(
        &mut violations,
        "scenario.expected_status",
        &manifest.expected_status,
    );
    require_non_empty(
        &mut violations,
        "scenario.expected_capability",
        &manifest.expected_capability,
    );
    if manifest
        .expected_operation
        .as_deref()
        .unwrap_or("")
        .is_empty()
        && manifest.expected_status == "applied"
    {
        violations.push(contract_violation(
            "scenario.expected_operation",
            "applied AST patch scenarios must name the expected operation",
        ));
    }
    let status = scenario_benchmark_status(&violations);
    Ok(RustScenarioBenchmarkReceipt {
        root: root.to_path_buf(),
        scenario,
        benchmark,
        status,
        violations,
    })
}

fn read_toml<T>(path: &Path) -> Result<T, RustScenarioBenchmarkError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))?;
    toml::from_str(&text).map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))
}

fn read_json<T>(path: &Path) -> Result<T, RustScenarioBenchmarkError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))?;
    serde_json::from_str(&text)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))
}

fn scenario_benchmark_status(
    violations: &[RustScenarioBenchmarkViolation],
) -> RustScenarioBenchmarkStatus {
    if violations
        .iter()
        .any(|violation| violation.kind == RustScenarioBenchmarkViolationKind::Contract)
    {
        return RustScenarioBenchmarkStatus::Invalid;
    }
    if violations.is_empty() {
        RustScenarioBenchmarkStatus::Pass
    } else {
        RustScenarioBenchmarkStatus::Fail
    }
}

fn scenario_benchmark_suite_status(
    receipts: &[RustScenarioBenchmarkReceipt],
    violations: &[RustScenarioBenchmarkViolation],
) -> RustScenarioBenchmarkStatus {
    if !violations.is_empty()
        || receipts
            .iter()
            .any(|receipt| receipt.status == RustScenarioBenchmarkStatus::Invalid)
    {
        return RustScenarioBenchmarkStatus::Invalid;
    }
    if receipts
        .iter()
        .any(|receipt| receipt.status == RustScenarioBenchmarkStatus::Fail)
    {
        return RustScenarioBenchmarkStatus::Fail;
    }
    RustScenarioBenchmarkStatus::Pass
}

fn scenario_benchmark_violations(
    scenario: &RustScenarioMetadata,
    benchmark: &RustScenarioBenchmarkContract,
) -> Vec<RustScenarioBenchmarkViolation> {
    let mut violations = Vec::new();
    require_non_empty(&mut violations, "scenario.id", &scenario.id);
    require_non_empty(&mut violations, "scenario.title", &scenario.title);
    require_non_empty(&mut violations, "scenario.agent_goal", &scenario.agent_goal);
    require_non_empty(&mut violations, "scenario.inputs", &scenario.inputs);
    require_non_empty(&mut violations, "scenario.expected", &scenario.expected);
    if scenario.policy_ids.is_empty() {
        violations.push(contract_violation(
            "scenario.policy_ids",
            "at least one policy id is required",
        ));
    }
    require_non_empty(&mut violations, "benchmark.harness", &benchmark.harness);
    require_supported_harness(&mut violations, &benchmark.harness);
    require_benchmark_entry(&mut violations, benchmark);
    if benchmark_entry_targets_contract_gate(benchmark) {
        violations.push(contract_violation(
            "benchmark.entry",
            "benchmark entry must name a focused Rust test or bench case, not the scenario benchmark contract gate",
        ));
    }
    require_non_empty(
        &mut violations,
        "benchmark.target_rationale",
        &benchmark.target_rationale,
    );
    require_positive(
        &mut violations,
        "benchmark.target_total",
        benchmark.target_total.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.max_total",
        benchmark.max_total.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.regression_budget",
        benchmark.regression_budget.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.memory_budget_bytes",
        benchmark.memory_budget_bytes.as_u64() == 0,
    );
    if benchmark.observed_timings.is_empty() {
        violations.push(contract_violation(
            "benchmark.observed_timings",
            "at least one timing phase is required",
        ));
    }
    if benchmark.target_total > benchmark.max_total {
        violations.push(contract_violation(
            "benchmark.target_total",
            "target_total must be less than or equal to max_total",
        ));
    }
    if benchmark.max_total > RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL {
        violations.push(contract_violation(
            "benchmark.max_total",
            &format!(
                "max_total must be <= {RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL} for the hard gate",
            ),
        ));
    }
    if benchmark.observed_total > benchmark.max_total {
        violations.push(RustScenarioBenchmarkViolation {
            kind: RustScenarioBenchmarkViolationKind::Performance,
            field: "benchmark.observed_total".to_string(),
            message: format!(
                "observed {} exceeds max {}",
                benchmark.observed_total, benchmark.max_total
            ),
        });
    }
    if benchmark.observed_memory_bytes > benchmark.memory_budget_bytes {
        violations.push(RustScenarioBenchmarkViolation {
            kind: RustScenarioBenchmarkViolationKind::Memory,
            field: "benchmark.observed_memory_bytes".to_string(),
            message: format!(
                "observed {} bytes exceeds budget {} bytes",
                benchmark.observed_memory_bytes, benchmark.memory_budget_bytes
            ),
        });
    }
    violations
}

fn require_non_empty(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    value: &str,
) {
    if value.trim().is_empty() {
        violations.push(contract_violation(field, "field must not be empty"));
    }
}

fn require_positive(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    is_zero: bool,
) {
    if is_zero {
        violations.push(contract_violation(field, "field must be greater than zero"));
    }
}

fn require_supported_harness(violations: &mut Vec<RustScenarioBenchmarkViolation>, harness: &str) {
    if harness.trim().is_empty() {
        return;
    }
    if !matches!(
        harness.trim(),
        "libtest" | "criterion" | "divan" | "iai-callgrind"
    ) {
        violations.push(contract_violation(
            "benchmark.harness",
            "harness must be one of libtest, criterion, divan, or iai-callgrind",
        ));
    }
}

fn require_benchmark_entry(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    benchmark: &RustScenarioBenchmarkContract,
) {
    let test = non_empty_opt(benchmark.test.as_deref());
    let bench = non_empty_opt(benchmark.bench.as_deref());
    let case = non_empty_opt(benchmark.case.as_deref());

    match (test, bench) {
        (None, None) => violations.push(contract_violation(
            "benchmark.entry",
            "set either test for libtest gates or bench plus case for benchmark harnesses",
        )),
        (Some(_), Some(_)) => violations.push(contract_violation(
            "benchmark.entry",
            "set either test or bench, not both",
        )),
        (Some(_), None) => {
            if benchmark.harness.trim() != "libtest" {
                violations.push(contract_violation(
                    "benchmark.test",
                    "test entries must use harness = \"libtest\"",
                ));
            }
            if case.is_some() {
                violations.push(contract_violation(
                    "benchmark.case",
                    "case is only valid with bench targets",
                ));
            }
        }
        (None, Some(_)) => {
            if benchmark.harness.trim() == "libtest" {
                violations.push(contract_violation(
                    "benchmark.bench",
                    "bench entries must use criterion, divan, or iai-callgrind",
                ));
            }
            if case.is_none() {
                violations.push(contract_violation(
                    "benchmark.case",
                    "bench entries must name a focused benchmark case, group, or function",
                ));
            }
        }
    }
}

fn non_empty_opt(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn contract_violation(field: &str, message: &str) -> RustScenarioBenchmarkViolation {
    RustScenarioBenchmarkViolation {
        kind: RustScenarioBenchmarkViolationKind::Contract,
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn ast_patch_scenario_title(root: &Path) -> String {
    let name = root
        .file_name()
        .map(|file_name| file_name.to_string_lossy())
        .unwrap_or_else(|| "ast_patch_scenario".into());
    format!("AST patch scenario {name}")
}
