//! Agent-facing stability picture derived from project-owned configuration.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::path::display_project_path;

use super::model::{
    RustVerificationPlan, RustVerificationStabilityPictureConfig, RustVerificationTaskState,
};
use super::stability::{RustVerificationStabilityIndex, build_rust_verification_stability_index};

/// Configured stability picture for agent planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityPicture {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Project-owned stability picture requirements.
    pub config: RustVerificationStabilityPictureConfig,
    /// Stability owner records included in this picture.
    pub records: Vec<RustVerificationStabilityPictureRecord>,
}

impl RustVerificationStabilityPicture {
    /// Return whether this picture has no stability records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return records that still need agent action.
    #[must_use]
    pub fn actionable_records(&self) -> Vec<&RustVerificationStabilityPictureRecord> {
        self.records
            .iter()
            .filter(|record| record.state.is_active() || !record.missing_evidence_keys.is_empty())
            .collect()
    }
}

/// One owner-level stability picture record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityPictureRecord {
    /// Stability task fingerprint.
    pub fingerprint: String,
    /// Current task state.
    pub state: RustVerificationTaskState,
    /// Cargo package root that owns this stability task.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Required evidence keys after applying the project-owned picture config.
    pub required_evidence_keys: Vec<String>,
    /// Missing configured evidence keys for this owner.
    pub missing_evidence_keys: Vec<String>,
    /// Agent-facing next actions derived from the configured picture.
    pub next_actions: Vec<String>,
}

/// Build a configured stability picture from a verification plan.
#[must_use]
pub fn build_rust_verification_stability_picture(
    plan: &RustVerificationPlan,
    config: &RustVerificationStabilityPictureConfig,
) -> RustVerificationStabilityPicture {
    let index = build_rust_verification_stability_index(plan);
    build_rust_verification_stability_picture_from_index(&index, config)
}

/// Build a configured stability picture from an existing stability index.
#[must_use]
pub fn build_rust_verification_stability_picture_from_index(
    index: &RustVerificationStabilityIndex,
    config: &RustVerificationStabilityPictureConfig,
) -> RustVerificationStabilityPicture {
    let required_keys = config
        .required_receipt_evidence_keys()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let records = index
        .records
        .iter()
        .map(|record| {
            let missing_evidence_keys = required_keys
                .iter()
                .filter(|key| record.receipt_evidence_value(key).is_none())
                .cloned()
                .collect::<Vec<_>>();
            RustVerificationStabilityPictureRecord {
                fingerprint: record.fingerprint.clone(),
                state: record.state,
                package_root: record.package_root.clone(),
                owner_path: record.owner_path.clone(),
                required_evidence_keys: required_keys.clone(),
                next_actions: stability_picture_next_actions(config, &missing_evidence_keys),
                missing_evidence_keys,
            }
        })
        .collect();
    RustVerificationStabilityPicture {
        project_root: index.project_root.clone(),
        config: config.clone(),
        records,
    }
}

/// Render a configured stability picture for agent planning.
#[must_use]
pub fn render_rust_verification_stability_picture(
    picture: &RustVerificationStabilityPicture,
) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "[stability-picture] records={} actionable={} axes={}",
        picture.records.len(),
        picture.actionable_records().len(),
        picture.config.required_receipt_evidence_keys().join(",")
    );
    if let Some(iterations) = picture.config.min_iterations {
        let _ = writeln!(rendered, "   |min_iterations: {iterations}");
    }
    if let Some(seconds) = picture.config.min_duration_seconds {
        let _ = writeln!(rendered, "   |min_duration_seconds: {seconds}");
    }
    let display_root = if picture.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(picture.project_root.as_path())
    };
    for record in &picture.records {
        render_stability_picture_record(record, display_root, &mut rendered);
    }
    rendered.trim_end().to_string()
}

/// Render a configured stability picture as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the picture cannot be encoded as JSON.
pub fn render_rust_verification_stability_picture_json(
    picture: &RustVerificationStabilityPicture,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(picture)
}

fn render_stability_picture_record(
    record: &RustVerificationStabilityPictureRecord,
    display_root: Option<&Path>,
    rendered: &mut String,
) {
    let display_root = display_root.unwrap_or(&record.package_root);
    let _ = writeln!(
        rendered,
        "   |owner: {} state={} fingerprint={}",
        display_project_path(display_root, &record.owner_path),
        record.state.as_str(),
        record.fingerprint
    );
    if !record.missing_evidence_keys.is_empty() {
        let _ = writeln!(
            rendered,
            "   |missing: {}",
            record.missing_evidence_keys.join(",")
        );
    }
    if !record.next_actions.is_empty() {
        let _ = writeln!(rendered, "   |next: {}", record.next_actions.join(";"));
    }
}

fn stability_picture_next_actions(
    config: &RustVerificationStabilityPictureConfig,
    missing_evidence_keys: &[String],
) -> Vec<String> {
    let mut actions = Vec::new();
    if missing_evidence_keys
        .iter()
        .any(|key| matches!(key.as_str(), "stability_command" | "iteration_window"))
    {
        let mut action = "add long-running simulation receipt".to_string();
        if let Some(iterations) = config.min_iterations {
            let _ = write!(action, " iterations>={iterations}");
        }
        if let Some(seconds) = config.min_duration_seconds {
            let _ = write!(action, " duration_s>={seconds}");
        }
        actions.push(action);
    }
    if missing_evidence_keys
        .iter()
        .any(|key| key == "latency_distribution")
    {
        actions.push("add performance interface latency distribution".to_string());
    }
    if missing_evidence_keys
        .iter()
        .any(|key| key == "resource_delta")
    {
        actions.push("add resource growth evidence".to_string());
    }
    if missing_evidence_keys
        .iter()
        .any(|key| key == "state_growth")
    {
        actions.push("add state growth evidence".to_string());
    }
    if missing_evidence_keys.iter().any(|key| key == "determinism") {
        actions.push("add deterministic replay evidence".to_string());
    }
    if missing_evidence_keys
        .iter()
        .any(|key| key == "stability_artifact")
    {
        actions.push("persist stability artifact".to_string());
    }
    actions
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}
