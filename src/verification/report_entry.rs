//! Agent entry projection for reading modular verification reports.

use std::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::analysis::RustVerificationAnalysisProfile;
use super::report::{
    RustVerificationReportArtifactRole, RustVerificationReportBundle,
    RustVerificationReportSidecarRole,
};
use super::report_manifest::{
    RustVerificationReportManifestCompatibility, RustVerificationReportManifestSchema,
    check_rust_verification_report_manifest_schema,
};
use super::report_select::{
    RustVerificationReportSelectionAdvice, build_rust_verification_report_selection_advice,
};
use super::report_write::{
    RustVerificationReportArtifactWriteReceipt, RustVerificationReportSidecarWriteReceipt,
    RustVerificationReportWriteReceipt,
};

/// One-step Agent advice for opening a verification report bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportEntryAdvice {
    /// Manifest schema advertised by the bundle.
    pub schema: RustVerificationReportManifestSchema,
    /// Compatibility of the advertised schema with this harness version.
    pub schema_compatibility: RustVerificationReportManifestCompatibility,
    /// First action an Agent should take before reading report payloads.
    pub action: RustVerificationReportEntryAction,
    /// Structured artifact selection advice, omitted when the schema is unsupported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<RustVerificationReportSelectionAdvice>,
    /// Persisted path for the selected first artifact, if a writer receipt was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_artifact: Option<RustVerificationReportEntryArtifact>,
    /// Persisted selection sidecar discovered from a writer receipt, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_sidecar: Option<RustVerificationReportEntrySidecar>,
}

/// Stable entry action for Agents reading modular verification reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReportEntryAction {
    /// The schema is unsupported; do not read artifact payloads.
    StopAndRefreshHarnessContract,
    /// A persisted sidecar should be read before individual artifacts.
    LoadSelectionAdviceSidecar,
    /// Read the selected first artifact from the manifest.
    ReadFirstArtifact,
    /// The manifest has no active report artifacts.
    NoActiveReportArtifacts,
}

/// Persisted artifact location attached to entry advice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportEntryArtifact {
    /// Stable report contract key.
    pub key: String,
    /// Agent-facing artifact role.
    pub role: RustVerificationReportArtifactRole,
    /// Persisted artifact path.
    pub path: PathBuf,
}

/// Persisted sidecar location attached to entry advice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportEntrySidecar {
    /// Stable sidecar contract key.
    pub key: String,
    /// Agent-facing sidecar role.
    pub role: RustVerificationReportSidecarRole,
    /// Persisted sidecar path.
    pub path: PathBuf,
}

impl RustVerificationReportEntryAction {
    /// Return a stable lowercase action label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StopAndRefreshHarnessContract => "stop_and_refresh_harness_contract",
            Self::LoadSelectionAdviceSidecar => "load_selection_advice_sidecar",
            Self::ReadFirstArtifact => "read_first_artifact",
            Self::NoActiveReportArtifacts => "no_active_report_artifacts",
        }
    }
}

/// Build entry advice without filesystem receipt context.
#[must_use]
pub fn build_rust_verification_report_entry_advice(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> RustVerificationReportEntryAdvice {
    build_rust_verification_report_entry_advice_with_receipt(bundle, analysis_profile, None)
}

/// Build entry advice with optional report writer receipt context.
#[must_use]
pub fn build_rust_verification_report_entry_advice_with_receipt(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
    receipt: Option<&RustVerificationReportWriteReceipt>,
) -> RustVerificationReportEntryAdvice {
    let schema_compatibility = check_rust_verification_report_manifest_schema(&bundle.schema);
    let selection_sidecar = receipt.and_then(selection_sidecar_from_receipt);
    let selection = schema_compatibility
        .is_supported()
        .then(|| build_rust_verification_report_selection_advice(bundle, analysis_profile));
    let first_artifact =
        receipt.and_then(|receipt| first_artifact_from_receipt(selection.as_ref(), receipt));
    let action = select_entry_action(
        &schema_compatibility,
        selection.as_ref(),
        &selection_sidecar,
    );
    RustVerificationReportEntryAdvice {
        schema: bundle.schema.clone(),
        schema_compatibility,
        action,
        selection,
        first_artifact,
        selection_sidecar,
    }
}

/// Render entry advice as JSON.
///
/// # Errors
///
/// Returns a serialization error if the advice cannot be encoded.
pub fn render_rust_verification_report_entry_advice_json(
    advice: &RustVerificationReportEntryAdvice,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(advice)
}

/// Render compact entry advice for windowed Agents.
#[must_use]
pub fn render_rust_verification_report_entry_advice(
    advice: &RustVerificationReportEntryAdvice,
) -> String {
    if !advice.schema_compatibility.is_supported() {
        return render_unsupported_entry_advice(advice);
    }

    let mut rendered = String::new();
    let first = advice.selection.as_ref().and_then(|selection| {
        selection
            .first
            .as_ref()
            .map(|first| (first, selection.reason.as_str()))
    });
    let _ = write!(
        rendered,
        "[verify-report-entry] schema={} state={} action={}",
        advice.schema.compact_label(),
        advice.schema_compatibility.as_str(),
        advice.action.as_str()
    );
    if let Some((first, reason)) = first {
        let _ = write!(
            rendered,
            " first={} role={} reason={}",
            first.key,
            first.role.as_str(),
            reason
        );
    } else {
        let reason = advice
            .selection
            .as_ref()
            .map(|selection| selection.reason.as_str())
            .unwrap_or("schema_unsupported");
        let _ = write!(rendered, " first=<none> reason={reason}");
    }
    if let Some(sidecar) = &advice.selection_sidecar {
        let _ = writeln!(rendered);
        let _ = write!(
            rendered,
            "   |sidecar: key={} role={} path={}",
            sidecar.key,
            sidecar.role.as_str(),
            sidecar.path.display()
        );
    }
    if let Some(artifact) = &advice.first_artifact {
        let _ = writeln!(rendered);
        let _ = write!(
            rendered,
            "   |artifact: key={} role={} path={}",
            artifact.key,
            artifact.role.as_str(),
            artifact.path.display()
        );
    }
    if let Some(selection) = &advice.selection {
        render_entry_order_line(selection, &mut rendered);
    }
    rendered
}

fn select_entry_action(
    compatibility: &RustVerificationReportManifestCompatibility,
    selection: Option<&RustVerificationReportSelectionAdvice>,
    sidecar: &Option<RustVerificationReportEntrySidecar>,
) -> RustVerificationReportEntryAction {
    if !compatibility.is_supported() {
        return RustVerificationReportEntryAction::StopAndRefreshHarnessContract;
    }
    if selection
        .and_then(|selection| selection.first.as_ref())
        .is_none()
    {
        return RustVerificationReportEntryAction::NoActiveReportArtifacts;
    }
    if sidecar.is_some() {
        return RustVerificationReportEntryAction::LoadSelectionAdviceSidecar;
    }
    RustVerificationReportEntryAction::ReadFirstArtifact
}

fn selection_sidecar_from_receipt(
    receipt: &RustVerificationReportWriteReceipt,
) -> Option<RustVerificationReportEntrySidecar> {
    receipt
        .sidecar_paths
        .iter()
        .find(|sidecar| sidecar.role == RustVerificationReportSidecarRole::SelectionAdvice)
        .map(RustVerificationReportEntrySidecar::from)
}

fn first_artifact_from_receipt(
    selection: Option<&RustVerificationReportSelectionAdvice>,
    receipt: &RustVerificationReportWriteReceipt,
) -> Option<RustVerificationReportEntryArtifact> {
    let key = &selection?.first.as_ref()?.key;
    receipt
        .artifact_paths
        .iter()
        .find(|artifact| artifact.key == *key)
        .map(RustVerificationReportEntryArtifact::from)
}

fn render_unsupported_entry_advice(advice: &RustVerificationReportEntryAdvice) -> String {
    match &advice.schema_compatibility {
        RustVerificationReportManifestCompatibility::Supported => unreachable!(),
        RustVerificationReportManifestCompatibility::UnsupportedSchemaId { expected, actual } => {
            format!(
                "[verify-report-entry] state={} action={} expected={} actual={} reason=\"{}\"",
                advice.schema_compatibility.as_str(),
                advice.action.as_str(),
                expected,
                actual,
                advice.schema_compatibility.reason().expect("reason")
            )
        }
        RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion {
            expected,
            actual,
        } => {
            format!(
                "[verify-report-entry] state={} action={} expected={} actual={} reason=\"{}\"",
                advice.schema_compatibility.as_str(),
                advice.action.as_str(),
                expected,
                actual,
                advice.schema_compatibility.reason().expect("reason")
            )
        }
    }
}

fn render_entry_order_line(
    selection: &RustVerificationReportSelectionAdvice,
    rendered: &mut String,
) {
    if selection.order.is_empty() {
        return;
    }
    let ordered_keys = selection
        .order
        .iter()
        .map(|artifact| artifact.key.as_str())
        .collect::<Vec<_>>();
    let _ = writeln!(rendered);
    let _ = write!(rendered, "   |order: {}", ordered_keys.join(" -> "));
}

impl From<&RustVerificationReportSidecarWriteReceipt> for RustVerificationReportEntrySidecar {
    fn from(sidecar: &RustVerificationReportSidecarWriteReceipt) -> Self {
        Self {
            key: sidecar.key.clone(),
            role: sidecar.role,
            path: sidecar.path.clone(),
        }
    }
}

impl From<&RustVerificationReportArtifactWriteReceipt> for RustVerificationReportEntryArtifact {
    fn from(artifact: &RustVerificationReportArtifactWriteReceipt) -> Self {
        Self {
            key: artifact.key.clone(),
            role: artifact.role,
            path: artifact.path.clone(),
        }
    }
}
