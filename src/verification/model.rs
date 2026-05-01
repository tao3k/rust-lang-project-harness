//! Public verification contract types.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Verification skill family requested by the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationTaskKind {
    /// High-concurrency and latency-curve validation.
    Stress,
    /// Rust-native benchmark and allocation-regression validation.
    Performance,
    /// Dependency failure and recovery validation.
    Chaos,
    /// Common vulnerability and authorization-boundary probing.
    Security,
    /// Long-term structural drift and architecture regression validation.
    Regression,
    /// Profile/config responsibility needs parser-fact review.
    ResponsibilityReview,
}

impl RustVerificationTaskKind {
    /// Stable label used by compact renders.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stress => "stress",
            Self::Performance => "performance",
            Self::Chaos => "chaos",
            Self::Security => "security",
            Self::Regression => "regression",
            Self::ResponsibilityReview => "responsibility_review",
        }
    }
}

/// Code responsibility declared by the embedding project or by an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustOwnerResponsibility {
    /// Pure computation or data-shape logic with no runtime side effects.
    PureDomainLogic,
    /// Public API, route, command, or integration surface.
    PublicApi,
    /// Calls a database, network service, filesystem, queue, or other dependency.
    ExternalDependency,
    /// Persists or migrates runtime state.
    Persistence,
    /// Authentication, authorization, secret, or trust-boundary logic.
    SecurityBoundary,
    /// Latency-sensitive path where p50/p99/p999 drift matters.
    LatencySensitive,
    /// Availability-sensitive path that must degrade and recover predictably.
    AvailabilityCritical,
}

impl RustOwnerResponsibility {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PureDomainLogic => "pure_domain_logic",
            Self::PublicApi => "public_api",
            Self::ExternalDependency => "external_dependency",
            Self::Persistence => "persistence",
            Self::SecurityBoundary => "security_boundary",
            Self::LatencySensitive => "latency_sensitive",
            Self::AvailabilityCritical => "availability_critical",
        }
    }
}

/// Preferred lifecycle moment for running the external verification skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationPhase {
    /// Run after ordinary unit/integration tests are green.
    AfterUnitTestsPass,
    /// Run before release or merge.
    BeforeRelease,
    /// Run on scheduled architecture health checks.
    ScheduledRegression,
    /// Review profile/config responsibility before trusting derived obligations.
    BeforeVerification,
}

impl RustVerificationPhase {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::AfterUnitTestsPass => "after_unit_tests_pass",
            Self::BeforeRelease => "before_release",
            Self::ScheduledRegression => "scheduled_regression",
            Self::BeforeVerification => "before_verification",
        }
    }
}

/// Agent-visible verification task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationTaskState {
    /// No matching passed receipt or complete waiver exists.
    Pending,
    /// A matching receipt reported success for the current task fingerprint.
    Satisfied,
    /// A matching receipt reported failure for the current task fingerprint.
    Failed,
    /// A matching complete waiver suppresses the current task fingerprint.
    Waived,
}

impl RustVerificationTaskState {
    pub(crate) const fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::Failed)
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Satisfied => "satisfied",
            Self::Failed => "failed",
            Self::Waived => "waived",
        }
    }
}

/// Result status reported by an external verification skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReceiptStatus {
    /// The external skill completed and the obligation is satisfied.
    Passed,
    /// The external skill completed and found a regression or risk.
    Failed,
}

/// Small parser-fact or profile-fact line attached to a verification task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationEvidence {
    /// Fact category.
    pub label: String,
    /// Compact fact value.
    pub value: String,
}

impl RustVerificationEvidence {
    /// Build one compact evidence fact.
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Structured evidence field required from an external verification skill.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationRequirement {
    /// Stable machine-readable requirement key.
    pub key: String,
    /// Compact agent-readable requirement description.
    pub description: String,
}

impl RustVerificationRequirement {
    /// Build one structured verification requirement.
    #[must_use]
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }
}

/// Configurable execution contract for one verification task family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationTaskContract {
    /// Suggested lifecycle phase for the external skill.
    pub phase: RustVerificationPhase,
    /// Receipt contract expected from the external skill.
    pub required_receipt: String,
    /// Structured evidence fields expected from the external skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_evidence: Vec<RustVerificationRequirement>,
}

impl RustVerificationTaskContract {
    /// Build one verification task contract.
    #[must_use]
    pub fn new(
        phase: RustVerificationPhase,
        required_receipt: impl Into<String>,
        required_evidence: impl IntoIterator<Item = RustVerificationRequirement>,
    ) -> Self {
        Self {
            phase,
            required_receipt: required_receipt.into(),
            required_evidence: required_evidence.into_iter().collect(),
        }
    }
}

/// Configured Agent skill adapter for one verification task family.
///
/// A binding means the embedding project already knows how to dispatch that
/// skill family, so compact renders can stay quiet and emit only the scheduler
/// hint instead of repeating contract text every run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationSkillBinding {
    /// Stable local or external skill id.
    pub skill_id: String,
    /// Optional adapter name such as `criterion`, `divan`, `iai-callgrind`,
    /// `k6`, or `semgrep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

impl RustVerificationSkillBinding {
    /// Build a configured skill binding.
    #[must_use]
    pub fn new(skill_id: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            adapter: None,
        }
    }

    /// Attach an adapter label for the configured skill.
    #[must_use]
    pub fn with_adapter(mut self, adapter: impl Into<String>) -> Self {
        self.adapter = Some(adapter.into());
        self
    }

    pub(crate) fn is_configured(&self) -> bool {
        !self.skill_id.trim().is_empty()
    }

    pub(crate) fn compact_label(&self) -> String {
        self.adapter
            .as_deref()
            .map(str::trim)
            .filter(|adapter| !adapter.is_empty())
            .map_or_else(
                || self.skill_id.clone(),
                |adapter| format!("{}@{adapter}", self.skill_id),
            )
    }
}

/// Compact contract that explains how an Agent skill binding is executed.
///
/// Bindings keep the default verification render quiet. Descriptors are the
/// optional reasoning-tree node an agent can expand when it needs the adapter's
/// execution standard without loading a long Markdown skill.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationSkillDescriptor {
    /// Stable local or external skill id.
    pub skill_id: String,
    /// Optional adapter name such as `criterion`, `divan`, `iai-callgrind`,
    /// `k6`, or `semgrep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Tool or runtime family used by the adapter.
    pub tool: String,
    /// Compact command template.
    pub command: String,
    /// Short pass/fail standard.
    pub standard: String,
    /// Inputs the Agent must resolve before dispatch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
    /// Criteria that make the run pass.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pass_criteria: Vec<String>,
    /// Receipt fields expected after the run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_fields: Vec<String>,
}

impl RustVerificationSkillDescriptor {
    /// Build a descriptor for a configured skill.
    #[must_use]
    pub fn new(skill_id: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            adapter: None,
            tool: String::new(),
            command: String::new(),
            standard: String::new(),
            required_inputs: Vec::new(),
            pass_criteria: Vec::new(),
            receipt_fields: Vec::new(),
        }
    }

    /// Built-in compact descriptor for the k6 stress adapter.
    ///
    /// The contract intentionally stays short: k6 scenarios define load shape,
    /// thresholds define pass/fail, and the receipt records the latency and SLA
    /// fields the harness already requires.
    #[must_use]
    pub fn k6_stress() -> Self {
        Self::new("rust-verification-stress")
            .with_adapter("k6")
            .with_tool("k6")
            .with_command("k6 run <script>")
            .with_standard("scenarios define load shape; thresholds define pass/fail")
            .with_required_inputs(["script", "target_url", "scenario", "thresholds"])
            .with_pass_criteria(["exit=0", "thresholds=pass"])
            .with_receipt_fields(["p50", "p99", "p999", "load_steps", "sla_result", "artifact"])
    }

    /// Built-in compact descriptor for Criterion-based Rust performance checks.
    ///
    /// Criterion is the statistics-oriented Rust benchmark adapter. Use it for
    /// code-level latency, throughput, and allocation-regression evidence rather
    /// than service-boundary stress tests.
    #[must_use]
    pub fn criterion_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("criterion")
            .with_tool("criterion")
            .with_command("cargo bench")
            .with_standard("statistical benchmark baseline detects runtime regression")
            .with_required_inputs(["bench_target", "baseline", "regression_threshold"])
            .with_pass_criteria(["regression<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
            ])
    }

    /// Built-in compact descriptor for Divan-based Rust performance checks.
    ///
    /// Divan is a modern Rust benchmark adapter over `cargo bench`; keep it in
    /// the Rust-native performance lane rather than the service stress lane.
    #[must_use]
    pub fn divan_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("divan")
            .with_tool("divan")
            .with_command("cargo bench")
            .with_standard("sampled Rust benchmark summary stays within regression threshold")
            .with_required_inputs(["bench_target", "baseline", "regression_threshold"])
            .with_pass_criteria(["median_or_mean_delta<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
                "samples",
                "iters",
            ])
    }

    /// Built-in compact descriptor for iai-callgrind Rust performance checks.
    ///
    /// iai-callgrind is the deterministic CI-oriented adapter for instruction,
    /// cache, and allocation profiles. It complements wall-clock benchmarks when
    /// the Agent needs lower-noise regression evidence.
    #[must_use]
    pub fn iai_callgrind_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("iai-callgrind")
            .with_tool("iai-callgrind")
            .with_command("cargo bench")
            .with_standard("instruction/cache/allocation metrics stay within regression threshold")
            .with_required_inputs(["bench_target", "baseline", "metric", "regression_threshold"])
            .with_pass_criteria(["metric_delta<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
                "instructions",
                "cache_misses",
            ])
    }

    /// Attach an adapter label for this descriptor.
    #[must_use]
    pub fn with_adapter(mut self, adapter: impl Into<String>) -> Self {
        self.adapter = Some(adapter.into());
        self
    }

    /// Set the tool family.
    #[must_use]
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = tool.into();
        self
    }

    /// Set the command template.
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    /// Set the compact execution standard.
    #[must_use]
    pub fn with_standard(mut self, standard: impl Into<String>) -> Self {
        self.standard = standard.into();
        self
    }

    /// Set required adapter inputs.
    #[must_use]
    pub fn with_required_inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_inputs = inputs.into_iter().map(Into::into).collect();
        self
    }

    /// Set pass criteria.
    #[must_use]
    pub fn with_pass_criteria<I, S>(mut self, criteria: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.pass_criteria = criteria.into_iter().map(Into::into).collect();
        self
    }

    /// Set receipt fields.
    #[must_use]
    pub fn with_receipt_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.receipt_fields = fields.into_iter().map(Into::into).collect();
        self
    }

    pub(crate) fn compact_label(&self) -> String {
        self.adapter
            .as_deref()
            .map(str::trim)
            .filter(|adapter| !adapter.is_empty())
            .map_or_else(
                || self.skill_id.clone(),
                |adapter| format!("{}@{adapter}", self.skill_id),
            )
    }

    pub(crate) fn fingerprint_material(&self) -> String {
        format!(
            "tool={};command={};standard={};inputs={};pass={};receipt={}",
            self.tool,
            self.command,
            self.standard,
            self.required_inputs.join(","),
            self.pass_criteria.join(","),
            self.receipt_fields.join(",")
        )
    }
}

/// Verification task generated from parser facts and optional profile hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationTask {
    /// Stable fingerprint for this exact obligation and parser evidence.
    pub fingerprint: String,
    /// Verification skill family.
    pub kind: RustVerificationTaskKind,
    /// Whether the task is still active.
    pub state: RustVerificationTaskState,
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path, or hinted path when the hint cannot be matched.
    pub owner_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// One-based source line when the triggering parser fact has a line.
    pub line: Option<usize>,
    /// Suggested lifecycle phase.
    pub phase: RustVerificationPhase,
    /// Why an agent should run or resolve this task.
    pub reason: String,
    /// Receipt contract expected from the external skill.
    pub required_receipt: String,
    /// Configured skill adapter for quiet dispatch, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_binding: Option<RustVerificationSkillBinding>,
    /// Contract descriptor key for expanding a configured skill, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_contract_ref: Option<String>,
    /// Structured evidence fields expected from the external skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_evidence: Vec<RustVerificationRequirement>,
    /// Parser/profile evidence used to produce the task.
    pub evidence: Vec<RustVerificationEvidence>,
    /// Why supplied resolution inputs did not clear the task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolution_notes: Vec<RustVerificationResolutionNote>,
    /// Matching receipt summary, when one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_summary: Option<String>,
    /// Structured receipt evidence copied from the matching external skill run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_evidence: Vec<RustVerificationEvidence>,
    /// Matching receipt artifact URI or local path, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_evidence_uri: Option<String>,
    /// Matching receipt timestamp, when supplied by the external skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_observed_at: Option<String>,
    /// Matching waiver reason, when one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver_reason: Option<String>,
}

impl RustVerificationTask {
    /// Return whether this task should still be rendered as an active reminder.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

/// Compact explanation for a receipt or waiver that did not clear a task.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationResolutionNote {
    /// Resolution input category.
    pub label: String,
    /// Why it did not clear the task.
    pub detail: String,
}

impl RustVerificationResolutionNote {
    /// Build one resolution note.
    #[must_use]
    pub fn new(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
        }
    }
}

/// Evidence receipt produced by an external skill run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReceipt {
    /// Fingerprint of the task that was verified.
    pub task_fingerprint: String,
    /// Verification skill family that produced the receipt.
    pub kind: RustVerificationTaskKind,
    /// Skill result.
    pub status: RustVerificationReceiptStatus,
    /// Compact result summary.
    pub summary: String,
    /// Optional evidence artifact URI or local path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_uri: Option<String>,
    /// Structured evidence emitted by the external skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<RustVerificationEvidence>,
    /// Optional timestamp supplied by the external skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
}

impl RustVerificationReceipt {
    /// Build a passed receipt for one current task fingerprint.
    #[must_use]
    pub fn passed(task_fingerprint: impl Into<String>, kind: RustVerificationTaskKind) -> Self {
        Self {
            task_fingerprint: task_fingerprint.into(),
            kind,
            status: RustVerificationReceiptStatus::Passed,
            summary: "passed".to_string(),
            evidence_uri: None,
            evidence: Vec::new(),
            observed_at: None,
        }
    }

    /// Build a failed receipt for one current task fingerprint.
    #[must_use]
    pub fn failed(
        task_fingerprint: impl Into<String>,
        kind: RustVerificationTaskKind,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            task_fingerprint: task_fingerprint.into(),
            kind,
            status: RustVerificationReceiptStatus::Failed,
            summary: summary.into(),
            evidence_uri: None,
            evidence: Vec::new(),
            observed_at: None,
        }
    }

    /// Attach one structured evidence field.
    #[must_use]
    pub fn with_evidence(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.evidence
            .push(RustVerificationEvidence::new(label, value));
        self
    }

    /// Attach an evidence artifact URI or local path.
    #[must_use]
    pub fn with_evidence_uri(mut self, evidence_uri: impl Into<String>) -> Self {
        self.evidence_uri = Some(evidence_uri.into());
        self
    }

    /// Attach an observed-at timestamp supplied by the external skill.
    #[must_use]
    pub fn with_observed_at(mut self, observed_at: impl Into<String>) -> Self {
        self.observed_at = Some(observed_at.into());
        self
    }
}

/// Explicit suppression for a current verification task fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationWaiver {
    /// Fingerprint of the task being waived.
    pub task_fingerprint: String,
    /// Accountable owner for the waiver.
    pub owner: String,
    /// Why this verification is intentionally not required now.
    pub reason: String,
    /// Expiry string supplied by the embedding project.
    pub expires_at: String,
}

impl RustVerificationWaiver {
    /// Build a waiver for one current task fingerprint.
    #[must_use]
    pub fn new(
        task_fingerprint: impl Into<String>,
        owner: impl Into<String>,
        reason: impl Into<String>,
        expires_at: impl Into<String>,
    ) -> Self {
        Self {
            task_fingerprint: task_fingerprint.into(),
            owner: owner.into(),
            reason: reason.into(),
            expires_at: expires_at.into(),
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        !self.owner.trim().is_empty()
            && !self.reason.trim().is_empty()
            && !self.expires_at.trim().is_empty()
    }
}

/// Configurable responsibility hint used to map parser owners to skill duties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationProfileHint {
    /// Owner file path. Relative paths are resolved against each package root.
    pub owner_path: PathBuf,
    /// Declared responsibilities for this owner.
    pub responsibilities: BTreeSet<RustOwnerResponsibility>,
    /// Explicit verification task kinds for this owner.
    ///
    /// `None` keeps the policy-derived responsibility mapping. `Some(empty)`
    /// means this owner intentionally has no external verification task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_kinds: Option<BTreeSet<RustVerificationTaskKind>>,
    /// Owner-local contract overrides. These win over global policy overrides.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_contract_overrides: BTreeMap<RustVerificationTaskKind, RustVerificationTaskContract>,
    /// Optional compact rationale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

impl RustVerificationProfileHint {
    /// Build a profile hint for one owner path.
    #[must_use]
    pub fn new<I>(owner_path: impl Into<PathBuf>, responsibilities: I) -> Self
    where
        I: IntoIterator<Item = RustOwnerResponsibility>,
    {
        Self {
            owner_path: owner_path.into(),
            responsibilities: responsibilities.into_iter().collect(),
            task_kinds: None,
            task_contract_overrides: BTreeMap::new(),
            rationale: None,
        }
    }

    /// Attach explicit verification task kinds for this owner.
    ///
    /// Passing an empty iterator suppresses profile-derived verification tasks
    /// for this owner without changing global responsibility defaults.
    #[must_use]
    pub fn with_task_kinds<I>(mut self, task_kinds: I) -> Self
    where
        I: IntoIterator<Item = RustVerificationTaskKind>,
    {
        self.task_kinds = Some(task_kinds.into_iter().collect());
        self
    }

    /// Mark this owner as having no external verification tasks.
    #[must_use]
    pub fn without_verification_tasks(mut self) -> Self {
        self.task_kinds = Some(BTreeSet::new());
        self
    }

    /// Attach an owner-local verification task contract override.
    #[must_use]
    pub fn with_task_contract(
        mut self,
        kind: RustVerificationTaskKind,
        contract: RustVerificationTaskContract,
    ) -> Self {
        self.task_contract_overrides.insert(kind, contract);
        self
    }

    /// Attach a compact rationale.
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }
}

/// Library-first verification configuration surface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPolicy {
    /// Responsibility hints supplied by the embedding project or by an agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile_hints: Vec<RustVerificationProfileHint>,
    /// Current receipts produced by external skill executions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<RustVerificationReceipt>,
    /// Current waivers that intentionally suppress active reminders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waivers: Vec<RustVerificationWaiver>,
    /// Verification task kinds disabled by the embedding project.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub disabled_task_kinds: BTreeSet<RustVerificationTaskKind>,
    /// Per-kind verification contract overrides.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_contract_overrides: BTreeMap<RustVerificationTaskKind, RustVerificationTaskContract>,
    /// Per-kind Agent skill bindings used for quiet dispatch.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skill_bindings: BTreeMap<RustVerificationTaskKind, RustVerificationSkillBinding>,
    /// Compact skill descriptors keyed by `skill_id` or `skill_id@adapter`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skill_descriptors: BTreeMap<String, RustVerificationSkillDescriptor>,
    /// Per-responsibility task mapping overrides.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub responsibility_task_overrides:
        BTreeMap<RustOwnerResponsibility, BTreeSet<RustVerificationTaskKind>>,
}

impl RustVerificationPolicy {
    /// Return whether this policy carries no explicit verification configuration.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profile_hints.is_empty()
            && self.receipts.is_empty()
            && self.waivers.is_empty()
            && self.disabled_task_kinds.is_empty()
            && self.task_contract_overrides.is_empty()
            && self.skill_bindings.is_empty()
            && self.skill_descriptors.is_empty()
            && self.responsibility_task_overrides.is_empty()
    }

    /// Return a policy with one profile hint appended.
    #[must_use]
    pub fn with_profile_hint(mut self, hint: RustVerificationProfileHint) -> Self {
        self.profile_hints.push(hint);
        self
    }

    /// Return a policy with one receipt appended.
    #[must_use]
    pub fn with_receipt(mut self, receipt: RustVerificationReceipt) -> Self {
        self.receipts.push(receipt);
        self
    }

    /// Return a policy with one waiver appended.
    #[must_use]
    pub fn with_waiver(mut self, waiver: RustVerificationWaiver) -> Self {
        self.waivers.push(waiver);
        self
    }

    /// Return a policy with one task kind disabled.
    #[must_use]
    pub fn with_disabled_task_kind(mut self, kind: RustVerificationTaskKind) -> Self {
        self.disabled_task_kinds.insert(kind);
        self
    }

    /// Return a policy with one verification task contract overridden.
    #[must_use]
    pub fn with_task_contract(
        mut self,
        kind: RustVerificationTaskKind,
        contract: RustVerificationTaskContract,
    ) -> Self {
        self.task_contract_overrides.insert(kind, contract);
        self
    }

    /// Return a policy with one verification skill binding configured.
    #[must_use]
    pub fn with_skill_binding(
        mut self,
        kind: RustVerificationTaskKind,
        binding: RustVerificationSkillBinding,
    ) -> Self {
        self.skill_bindings.insert(kind, binding);
        self
    }

    /// Return a policy with one verification skill descriptor configured.
    #[must_use]
    pub fn with_skill_descriptor(mut self, descriptor: RustVerificationSkillDescriptor) -> Self {
        self.skill_descriptors
            .insert(descriptor.compact_label(), descriptor);
        self
    }

    /// Return a policy with one responsibility mapped to explicit task kinds.
    #[must_use]
    pub fn with_responsibility_task_kinds<I>(
        mut self,
        responsibility: RustOwnerResponsibility,
        task_kinds: I,
    ) -> Self
    where
        I: IntoIterator<Item = RustVerificationTaskKind>,
    {
        self.responsibility_task_overrides
            .insert(responsibility, task_kinds.into_iter().collect());
        self
    }
}

/// Full verification plan for a parser-scoped project run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPlan {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// All generated tasks, including satisfied or waived tasks.
    pub tasks: Vec<RustVerificationTask>,
    /// Compact descriptors referenced by tasks in this plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_descriptors: Vec<RustVerificationSkillDescriptor>,
}

impl RustVerificationPlan {
    /// Return tasks that still require agent action.
    #[must_use]
    pub fn active_tasks(&self) -> Vec<&RustVerificationTask> {
        self.tasks.iter().filter(|task| task.is_active()).collect()
    }

    /// Return whether no active verification reminder remains.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.active_tasks().is_empty()
    }
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}
