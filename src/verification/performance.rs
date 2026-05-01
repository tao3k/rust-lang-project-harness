//! Structured performance-status retrieval for verification plans.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationTaskKind, RustVerificationTaskState,
};

/// Searchable performance verification state extracted from one plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPerformanceIndex {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Performance task records, including pending, failed, and satisfied runs.
    pub records: Vec<RustVerificationPerformanceRecord>,
}

impl RustVerificationPerformanceIndex {
    /// Return whether no performance task exists in this plan.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return records owned by one path.
    #[must_use]
    pub fn records_for_owner(
        &self,
        owner_path: impl AsRef<Path>,
    ) -> Vec<&RustVerificationPerformanceRecord> {
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
    ) -> Vec<&RustVerificationPerformanceRecord> {
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
    ) -> Vec<&RustVerificationPerformanceRecord> {
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
    ) -> Vec<&RustVerificationPerformanceRecord> {
        self.records
            .iter()
            .filter(|record| record.receipt_evidence_value(key).is_some())
            .collect()
    }
}

/// One searchable performance task record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPerformanceRecord {
    /// Stable task fingerprint.
    pub fingerprint: String,
    /// Current verification state for this performance task.
    pub state: RustVerificationTaskState,
    /// Suggested lifecycle phase.
    pub phase: RustVerificationPhase,
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// Configured performance skill label, such as `rust-verification-performance@criterion`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    /// Descriptor key for expanding the adapter contract, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    /// Required evidence keys from the performance task contract.
    pub required_evidence_keys: Vec<String>,
    /// Parser/profile facts that triggered the performance obligation.
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
    /// Structured benchmark/profiling evidence from the matching receipt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_evidence: Vec<RustVerificationEvidence>,
}

impl RustVerificationPerformanceRecord {
    /// Return one receipt evidence value by key.
    #[must_use]
    pub fn receipt_evidence_value(&self, key: &str) -> Option<&str> {
        self.receipt_evidence
            .iter()
            .find(|evidence| evidence.label == key)
            .map(|evidence| evidence.value.as_str())
    }

    /// Return required evidence keys not present in the matching receipt.
    #[must_use]
    pub fn missing_receipt_evidence_keys(&self) -> Vec<&str> {
        self.required_evidence_keys
            .iter()
            .filter(|key| self.receipt_evidence_value(key).is_none())
            .map(String::as_str)
            .collect()
    }
}

/// Build a structured performance-status index from a verification plan.
#[must_use]
pub fn build_rust_verification_performance_index(
    plan: &RustVerificationPlan,
) -> RustVerificationPerformanceIndex {
    let mut records = plan
        .tasks
        .iter()
        .filter(|task| task.kind == RustVerificationTaskKind::Performance)
        .map(|task| RustVerificationPerformanceRecord {
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
    RustVerificationPerformanceIndex {
        project_root: plan.project_root.clone(),
        records,
    }
}

/// Render performance-status records for agent retrieval.
#[must_use]
pub fn render_rust_verification_performance_index(
    index: &RustVerificationPerformanceIndex,
) -> String {
    let display_root = if index.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(index.project_root.as_path())
    };
    index
        .records
        .iter()
        .map(|record| render_performance_record(record, display_root))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render performance-status records as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the index cannot be encoded as JSON.
pub fn render_rust_verification_performance_index_json(
    index: &RustVerificationPerformanceIndex,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(index)
}

fn render_performance_record(
    record: &RustVerificationPerformanceRecord,
    display_root: Option<&Path>,
) -> String {
    let display_root = display_root.unwrap_or(&record.package_root);
    let mut rendered = format!(
        "[perf-state] {}\n",
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

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}
