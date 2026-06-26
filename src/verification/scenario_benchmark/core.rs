//! Scenario benchmark contracts for Rust harness fixtures.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::contract_gate::{
    benchmark_entry_targets_contract_gate, default_benchmark_toml_template,
};
use serde::Deserialize;
use serde::de::{self, Visitor};

const RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL: RustScenarioBenchmarkDuration =
    RustScenarioBenchmarkDuration(Duration::from_millis(500));

/// Scenario manifest format that requires a benchmark contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkManifestKind {
    /// Native Rust harness scenario contract stored in `scenario.toml`.
    ScenarioToml,
    /// CLI AST patch scenario contract stored in `scenario.json`.
    AstPatchScenarioJson,
}

impl RustScenarioBenchmarkManifestKind {
    /// Return the stable manifest kind token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScenarioToml => "scenario.toml",
            Self::AstPatchScenarioJson => "ast-patch-scenario.json",
        }
    }
}

/// One scenario root that must carry a benchmark contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkRequirement {
    /// Scenario root directory.
    pub root: PathBuf,
    /// Manifest kind discovered in this root.
    pub manifest_kind: RustScenarioBenchmarkManifestKind,
}

/// Validation receipt for all required scenario benchmark contracts in a crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkSuiteReceipt {
    /// Crate root used for scenario discovery.
    pub root: PathBuf,
    /// Required scenario roots discovered from fixture conventions.
    pub requirements: Vec<RustScenarioBenchmarkRequirement>,
    /// Successfully loaded per-scenario benchmark receipts.
    pub receipts: Vec<RustScenarioBenchmarkReceipt>,
    /// Suite-level contract violations, such as a missing `benchmark.toml`.
    pub violations: Vec<RustScenarioBenchmarkViolation>,
    /// Overall suite validation status.
    pub status: RustScenarioBenchmarkStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RustAstPatchScenarioManifest {
    mode: String,
    expected_status: String,
    expected_capability: String,
    #[serde(default)]
    expected_operation: Option<String>,
}

/// Agent-visible scenario metadata loaded from `scenario.toml`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RustScenarioMetadata {
    /// Stable scenario id used in snapshots and policy receipts.
    pub id: String,
    /// Human-readable scenario title.
    pub title: String,
    /// Policy ids exercised by this scenario.
    #[serde(default)]
    pub policy_ids: Vec<String>,
    /// Agent-facing goal that explains how the scenario should be used.
    pub agent_goal: String,
    /// Relative input fixture directory.
    pub inputs: String,
    /// Relative expected-output fixture directory.
    pub expected: String,
}

/// Duration used by Rust scenario benchmark contracts.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustScenarioBenchmarkDuration(pub Duration);

impl RustScenarioBenchmarkDuration {
    /// Return the raw duration.
    #[must_use]
    pub const fn as_duration(self) -> Duration {
        self.0
    }

    fn is_zero(self) -> bool {
        self.0.is_zero()
    }
}

impl<'de> Deserialize<'de> for RustScenarioBenchmarkDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(RustScenarioBenchmarkDurationVisitor)
    }
}

struct RustScenarioBenchmarkDurationVisitor;

impl Visitor<'_> for RustScenarioBenchmarkDurationVisitor {
    type Value = RustScenarioBenchmarkDuration;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Rust duration string such as 800ns, 75us, 1.2ms, or 1s")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        parse_rust_scenario_benchmark_duration(value).map_err(E::custom)
    }
}

impl fmt::Display for RustScenarioBenchmarkDuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.0)
    }
}

/// Byte count used by Rust scenario memory budget contracts.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct RustScenarioBenchmarkMemoryBytes(pub u64);

impl RustScenarioBenchmarkMemoryBytes {
    /// Return the raw byte count.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RustScenarioBenchmarkMemoryBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Scenario benchmark thresholds and observed receipts loaded from `benchmark.toml`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RustScenarioBenchmarkContract {
    /// Benchmark harness, such as libtest, criterion, divan, or iai-callgrind.
    pub harness: String,
    /// Focused libtest test function, when the perf gate is a Rust test.
    #[serde(default)]
    pub test: Option<String>,
    /// Cargo bench target name, when the perf gate is a benchmark target.
    #[serde(default)]
    pub bench: Option<String>,
    /// Focused benchmark case, group, or function inside a bench target.
    #[serde(default)]
    pub case: Option<String>,
    /// Insta snapshot name that freezes the receipt shape, when present.
    #[serde(default)]
    pub snapshot: Option<String>,
    /// Intended steady-state target duration.
    pub target_total: RustScenarioBenchmarkDuration,
    /// Hard maximum duration enforced by the scenario gate.
    pub max_total: RustScenarioBenchmarkDuration,
    /// Last observed total duration.
    pub observed_total: RustScenarioBenchmarkDuration,
    /// Allowed regression window before this scenario should be re-tuned.
    pub regression_budget: RustScenarioBenchmarkDuration,
    /// Hard memory budget enforced by the scenario gate.
    pub memory_budget_bytes: RustScenarioBenchmarkMemoryBytes,
    /// Last observed memory use.
    pub observed_memory_bytes: RustScenarioBenchmarkMemoryBytes,
    /// Agent-facing explanation for why the target is credible.
    pub target_rationale: String,
    /// Phase-level observed timings, normalized in snapshots.
    #[serde(default)]
    pub observed_timings: BTreeMap<String, RustScenarioBenchmarkDuration>,
}

impl RustScenarioBenchmarkContract {
    /// Return a compact, agent-facing benchmark entry label.
    #[must_use]
    pub fn bench_entry(&self) -> String {
        let mut parts = vec![format!("harness={}", self.harness.trim())];
        if let Some(test) = non_empty_opt(self.test.as_deref()) {
            parts.push(format!("test={test}"));
        }
        if let Some(bench) = non_empty_opt(self.bench.as_deref()) {
            parts.push(format!("bench={bench}"));
        }
        if let Some(case) = non_empty_opt(self.case.as_deref()) {
            parts.push(format!("case={case}"));
        }
        if let Some(snapshot) = non_empty_opt(self.snapshot.as_deref()) {
            parts.push(format!("snapshot={snapshot}"));
        }
        parts.join(" ")
    }
}

/// Validation receipt for one scenario benchmark fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkReceipt {
    /// Scenario root that owns `scenario.toml` and `benchmark.toml`.
    pub root: PathBuf,
    /// Agent-visible scenario metadata.
    pub scenario: RustScenarioMetadata,
    /// Numeric benchmark contract and observations.
    pub benchmark: RustScenarioBenchmarkContract,
    /// Overall validation status.
    pub status: RustScenarioBenchmarkStatus,
    /// Contract, performance, or memory violations.
    pub violations: Vec<RustScenarioBenchmarkViolation>,
}

/// Overall scenario benchmark validation status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkStatus {
    /// Contract and observed numbers satisfy the gate.
    Pass,
    /// Contract is valid but observed performance or memory exceeds the gate.
    Fail,
    /// Required contract metadata or thresholds are invalid.
    Invalid,
}

impl RustScenarioBenchmarkStatus {
    /// Return the stable lowercase status token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Invalid => "invalid",
        }
    }
}

/// One scenario benchmark validation violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkViolation {
    /// Violation class.
    pub kind: RustScenarioBenchmarkViolationKind,
    /// Stable field path that failed validation.
    pub field: String,
    /// Agent-facing explanation of the failed condition.
    pub message: String,
}

/// Violation class for scenario benchmark validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkViolationKind {
    /// Required metadata or threshold contract is invalid.
    Contract,
    /// Observed runtime exceeds the allowed duration gate.
    Performance,
    /// Observed memory exceeds the allowed memory gate.
    Memory,
}

impl RustScenarioBenchmarkViolationKind {
    /// Return the stable lowercase violation kind token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Performance => "performance",
            Self::Memory => "memory",
        }
    }
}

/// Error returned when a scenario benchmark contract cannot be read or parsed.
#[derive(Debug)]
pub struct RustScenarioBenchmarkError {
    path: PathBuf,
    message: String,
}

impl RustScenarioBenchmarkError {
    fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for RustScenarioBenchmarkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl Error for RustScenarioBenchmarkError {}

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

/// Discover every Rust harness scenario root that must carry `benchmark.toml`.
pub fn discover_required_rust_scenario_benchmarks(
    crate_root: impl AsRef<Path>,
) -> Result<Vec<RustScenarioBenchmarkRequirement>, RustScenarioBenchmarkError> {
    let crate_root = crate_root.as_ref();
    let mut requirements = Vec::new();
    collect_scenario_toml_requirements(
        &crate_root.join("tests").join("unit").join("scenarios"),
        &mut requirements,
    )?;
    collect_ast_patch_scenario_requirements(
        &crate_root
            .join("tests")
            .join("fixtures")
            .join("ast_patch_scenarios"),
        &mut requirements,
    )?;
    requirements.sort_by(|left, right| left.root.cmp(&right.root));
    Ok(requirements)
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

fn parse_rust_scenario_benchmark_duration(
    value: &str,
) -> Result<RustScenarioBenchmarkDuration, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("duration must not be empty".to_string());
    }
    let unit_start = value
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .ok_or_else(|| "duration must include a unit: ns, us, ms, or s".to_string())?;
    let (amount, unit) = value.split_at(unit_start);
    if amount.is_empty() {
        return Err("duration must include a numeric amount".to_string());
    }
    let nanos_per_unit = match unit {
        "ns" => 1,
        "us" | "\u{00b5}s" | "\u{03bc}s" => 1_000,
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        _ => {
            return Err(format!(
                "unsupported duration unit {unit:?}; use ns, us, ms, or s"
            ));
        }
    };
    let nanos = parse_decimal_duration_nanos(amount, nanos_per_unit)?;
    let nanos = u64::try_from(nanos)
        .map_err(|_| "duration exceeds std::time::Duration::from_nanos range".to_string())?;
    Ok(RustScenarioBenchmarkDuration(Duration::from_nanos(nanos)))
}

fn parse_decimal_duration_nanos(amount: &str, nanos_per_unit: u128) -> Result<u128, String> {
    let (whole, fraction) = amount
        .split_once('.')
        .map_or((amount, None), |(whole, fraction)| (whole, Some(fraction)));
    if whole.is_empty() && fraction.is_none_or(str::is_empty) {
        return Err("duration amount must contain digits".to_string());
    }
    let whole_nanos = if whole.is_empty() {
        0
    } else {
        whole
            .parse::<u128>()
            .map_err(|_| format!("invalid duration amount {amount:?}"))?
            .checked_mul(nanos_per_unit)
            .ok_or_else(|| "duration amount overflows nanoseconds".to_string())?
    };
    let Some(fraction) = fraction else {
        return Ok(whole_nanos);
    };
    if fraction.is_empty() || !fraction.chars().all(|character| character.is_ascii_digit()) {
        return Err(format!("invalid duration fraction {amount:?}"));
    }
    let scale = 10_u128
        .checked_pow(
            u32::try_from(fraction.len())
                .map_err(|_| "duration fraction is too precise".to_string())?,
        )
        .ok_or_else(|| "duration fraction is too precise".to_string())?;
    let fraction_units = fraction
        .parse::<u128>()
        .map_err(|_| format!("invalid duration fraction {amount:?}"))?;
    let fraction_nanos = fraction_units
        .checked_mul(nanos_per_unit)
        .ok_or_else(|| "duration fraction overflows nanoseconds".to_string())?;
    if fraction_nanos % scale != 0 {
        return Err(format!(
            "duration {amount:?} is more precise than nanoseconds"
        ));
    }
    whole_nanos
        .checked_add(fraction_nanos / scale)
        .ok_or_else(|| "duration amount overflows nanoseconds".to_string())
}

fn contract_violation(field: &str, message: &str) -> RustScenarioBenchmarkViolation {
    RustScenarioBenchmarkViolation {
        kind: RustScenarioBenchmarkViolationKind::Contract,
        field: field.to_string(),
        message: message.to_string(),
    }
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

fn collect_scenario_toml_requirements(
    root: &Path,
    requirements: &mut Vec<RustScenarioBenchmarkRequirement>,
) -> Result<(), RustScenarioBenchmarkError> {
    if !root.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root)
        .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    if root.join("scenario.toml").exists() {
        requirements.push(RustScenarioBenchmarkRequirement {
            root: root.to_path_buf(),
            manifest_kind: RustScenarioBenchmarkManifestKind::ScenarioToml,
        });
        return Ok(());
    }
    for entry in entries {
        if entry.is_dir() {
            collect_scenario_toml_requirements(&entry, requirements)?;
        }
    }
    Ok(())
}

fn collect_ast_patch_scenario_requirements(
    root: &Path,
    requirements: &mut Vec<RustScenarioBenchmarkRequirement>,
) -> Result<(), RustScenarioBenchmarkError> {
    if !root.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root)
        .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for entry in entries {
        if entry.is_dir() && entry.join("scenario.json").exists() {
            requirements.push(RustScenarioBenchmarkRequirement {
                root: entry,
                manifest_kind: RustScenarioBenchmarkManifestKind::AstPatchScenarioJson,
            });
        }
    }
    Ok(())
}

fn ast_patch_scenario_title(root: &Path) -> String {
    let name = root
        .file_name()
        .map(|file_name| file_name.to_string_lossy())
        .unwrap_or_else(|| "ast_patch_scenario".into());
    format!("AST patch scenario {name}")
}

fn display_suite_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
