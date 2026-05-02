//! Compact verification task state for source-controlled baselines.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{
    RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationTaskKind, RustVerificationTaskState,
};

/// Searchable verification state extracted from one plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationTaskIndex {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Compact task records, including security, performance, stress, chaos,
    /// regression, and responsibility-review obligations.
    pub records: Vec<RustVerificationTaskRecord>,
}

impl RustVerificationTaskIndex {
    /// Return whether no verification task exists in this plan.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return records in one verification state.
    #[must_use]
    pub fn records_in_state(
        &self,
        state: RustVerificationTaskState,
    ) -> Vec<&RustVerificationTaskRecord> {
        self.records
            .iter()
            .filter(|record| record.state == state)
            .collect()
    }

    /// Return records for one task kind.
    #[must_use]
    pub fn records_for_kind(
        &self,
        kind: RustVerificationTaskKind,
    ) -> Vec<&RustVerificationTaskRecord> {
        self.records
            .iter()
            .filter(|record| record.kind == kind)
            .collect()
    }
}

/// One compact verification task record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationTaskRecord {
    /// Stable task fingerprint.
    pub fingerprint: String,
    /// Verification skill family.
    pub kind: RustVerificationTaskKind,
    /// Current verification state.
    pub state: RustVerificationTaskState,
    /// Suggested lifecycle phase.
    pub phase: RustVerificationPhase,
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// One-based source line when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Configured skill label, such as `rust-verification-security@semgrep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Descriptor key for expanding the adapter contract, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    /// Required evidence keys from the task contract.
    pub required_evidence_keys: Vec<String>,
    /// Parser/profile facts that triggered the task.
    pub task_evidence: Vec<RustVerificationEvidence>,
    /// Matching receipt summary, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_summary: Option<String>,
    /// Matching receipt artifact URI or local path, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_evidence_uri: Option<String>,
    /// Matching receipt timestamp, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_observed_at: Option<String>,
}

/// Build a structured verification task index from a plan.
#[must_use]
pub fn build_rust_verification_task_index(
    plan: &RustVerificationPlan,
) -> RustVerificationTaskIndex {
    let mut records = plan
        .tasks
        .iter()
        .map(|task| RustVerificationTaskRecord {
            fingerprint: task.fingerprint.clone(),
            kind: task.kind,
            state: task.state,
            phase: task.phase,
            package_root: task.package_root.clone(),
            owner_path: task.owner_path.clone(),
            owner_namespace: task.owner_namespace.clone(),
            line: task.line,
            skill: task
                .skill_binding
                .as_ref()
                .map(|binding| binding.compact_label()),
            contract_ref: task.skill_contract_ref.clone(),
            required_evidence_keys: task
                .required_evidence
                .iter()
                .map(|requirement| requirement.key.clone())
                .collect(),
            task_evidence: task.evidence.clone(),
            receipt_summary: task.receipt_summary.clone(),
            receipt_evidence_uri: task.receipt_evidence_uri.clone(),
            receipt_observed_at: task.receipt_observed_at.clone(),
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    RustVerificationTaskIndex {
        project_root: plan.project_root.clone(),
        records,
    }
}

/// Render task records as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the index cannot be encoded as JSON.
pub fn render_rust_verification_task_index_json(
    index: &RustVerificationTaskIndex,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(index)
}

fn path_buf_is_empty(path: &std::path::Path) -> bool {
    path.as_os_str().is_empty()
}
