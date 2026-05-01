//! Public verification contract types.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Verification skill family requested by the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationTaskKind {
    /// High-concurrency and latency-curve validation.
    Stress,
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
    /// Parser/profile evidence used to produce the task.
    pub evidence: Vec<RustVerificationEvidence>,
    /// Matching receipt summary, when one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_summary: Option<String>,
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
            observed_at: None,
        }
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
            rationale: None,
        }
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
}

impl RustVerificationPolicy {
    /// Return whether this policy carries no explicit verification configuration.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profile_hints.is_empty()
            && self.receipts.is_empty()
            && self.waivers.is_empty()
            && self.disabled_task_kinds.is_empty()
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
}

/// Full verification plan for a parser-scoped project run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPlan {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// All generated tasks, including satisfied or waived tasks.
    pub tasks: Vec<RustVerificationTask>,
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
