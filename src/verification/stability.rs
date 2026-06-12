//! Searchable stability verification state.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::path::display_project_path;

use super::model::{
    RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationTaskKind, RustVerificationTaskState,
};

/// Searchable stability verification state extracted from one plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityIndex {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Stability task records, including pending, failed, and satisfied runs.
    pub records: Vec<RustVerificationStabilityRecord>,
}

impl RustVerificationStabilityIndex {
    /// Return whether no stability task exists in this plan.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return records owned by one path.
    #[must_use]
    pub fn records_for_owner(
        &self,
        owner_path: impl AsRef<Path>,
    ) -> Vec<&RustVerificationStabilityRecord> {
        let owner_path = owner_path.as_ref();
        self.records
            .iter()
            .filter(|record| {
                record.owner_path == owner_path
                    || record
                        .owner_path
                        .strip_prefix(&self.project_root)
                        .is_ok_and(|relative| relative == owner_path)
                    || record
                        .owner_path
                        .strip_prefix(&record.package_root)
                        .is_ok_and(|relative| relative == owner_path)
            })
            .collect()
    }

    /// Return records owned by one package root.
    #[must_use]
    pub fn records_for_package(
        &self,
        package_root: impl AsRef<Path>,
    ) -> Vec<&RustVerificationStabilityRecord> {
        let package_root = package_root.as_ref();
        self.records
            .iter()
            .filter(|record| {
                record.package_root == package_root
                    || record
                        .package_root
                        .strip_prefix(&self.project_root)
                        .is_ok_and(|relative| relative == package_root)
            })
            .collect()
    }

    /// Return records in one verification state.
    #[must_use]
    pub fn records_in_state(
        &self,
        state: RustVerificationTaskState,
    ) -> Vec<&RustVerificationStabilityRecord> {
        self.records
            .iter()
            .filter(|record| record.state == state)
            .collect()
    }

    /// Return records that carry one receipt evidence key.
    #[must_use]
    pub fn records_with_receipt_evidence(
        &self,
        key: &str,
    ) -> Vec<&RustVerificationStabilityRecord> {
        self.records
            .iter()
            .filter(|record| record.receipt_evidence_value(key).is_some())
            .collect()
    }
}

/// One searchable stability task record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityRecord {
    /// Stable task fingerprint.
    pub fingerprint: String,
    /// Current verification state for this stability task.
    pub state: RustVerificationTaskState,
    /// Suggested lifecycle phase.
    pub phase: RustVerificationPhase,
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// Configured stability skill label, when supplied by the embedding project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Descriptor key for expanding the adapter contract, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    /// Required evidence keys from the stability task contract.
    pub required_evidence_keys: Vec<String>,
    /// Parser/profile facts that triggered the stability obligation.
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
    /// Structured long-run stability evidence from the matching receipt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_evidence: Vec<RustVerificationEvidence>,
}

impl RustVerificationStabilityRecord {
    /// Return the required evidence keys that are absent from the receipt.
    #[must_use]
    pub fn missing_receipt_evidence_keys(&self) -> Vec<&str> {
        self.required_evidence_keys
            .iter()
            .filter_map(|key| {
                self.receipt_evidence_value(key)
                    .is_none()
                    .then_some(key.as_str())
            })
            .collect()
    }

    /// Return one receipt evidence value by key.
    #[must_use]
    pub fn receipt_evidence_value(&self, key: &str) -> Option<&str> {
        self.receipt_evidence
            .iter()
            .find(|evidence| evidence.label == key)
            .map(|evidence| evidence.value.as_str())
    }
}

/// Build a structured stability-status index from a verification plan.
#[must_use]
pub fn build_rust_verification_stability_index(
    plan: &RustVerificationPlan,
) -> RustVerificationStabilityIndex {
    let mut records = plan
        .tasks
        .iter()
        .filter(|task| task.kind == RustVerificationTaskKind::Stability)
        .map(|task| RustVerificationStabilityRecord {
            fingerprint: task.fingerprint.clone(),
            state: task.state,
            phase: task.phase,
            package_root: task.package_root.clone(),
            owner_path: task.owner_path.clone(),
            owner_namespace: task.owner_namespace.clone(),
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
            receipt_evidence: task.receipt_evidence.clone(),
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.owner_path
            .cmp(&right.owner_path)
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    RustVerificationStabilityIndex {
        project_root: plan.project_root.clone(),
        records,
    }
}

/// Render stability-status records for agent retrieval.
#[must_use]
pub fn render_rust_verification_stability_index(index: &RustVerificationStabilityIndex) -> String {
    let display_root = if index.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(index.project_root.as_path())
    };
    index
        .records
        .iter()
        .map(|record| render_stability_record(record, display_root))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render stability-status records as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the index cannot be encoded as JSON.
pub fn render_rust_verification_stability_index_json(
    index: &RustVerificationStabilityIndex,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(index)
}

fn render_stability_record(
    record: &RustVerificationStabilityRecord,
    display_root: Option<&Path>,
) -> String {
    let display_root = display_root.unwrap_or(&record.package_root);
    let mut rendered = format!(
        "[stability-state] {}\n",
        display_project_path(display_root, &record.owner_path)
    );
    if !record.owner_namespace.is_empty() {
        let _ = writeln!(rendered, "   |owner: {}", record.owner_namespace.join("/"));
    }
    let _ = write!(
        rendered,
        "   |state: {} phase={} fingerprint={}",
        record.state.as_str(),
        record.phase.as_str(),
        record.fingerprint
    );
    if let Some(skill) = &record.skill {
        let _ = write!(rendered, " skill={skill}");
    }
    if let Some(contract_ref) = &record.contract_ref {
        let _ = write!(rendered, " contract_ref={contract_ref}");
    }
    let _ = writeln!(rendered);
    if let Some(summary) = &record.receipt_summary {
        let _ = writeln!(rendered, "   |receipt: {summary}");
    }
    if let Some(observed_at) = &record.receipt_observed_at {
        let _ = writeln!(rendered, "   |observed_at: {observed_at}");
    }
    if !record.receipt_evidence.is_empty() {
        let evidence = record
            .receipt_evidence
            .iter()
            .map(|evidence| format!("{}={}", evidence.label, evidence.value))
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(rendered, "   |evidence: {evidence}");
    }
    let missing = record.missing_receipt_evidence_keys();
    if record.state.is_active() && !missing.is_empty() {
        let _ = writeln!(rendered, "   |missing: {}", missing.join(","));
    }
    if let Some(uri) = &record.receipt_evidence_uri {
        let _ = writeln!(rendered, "   |artifact: {uri}");
    }
    rendered
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}
