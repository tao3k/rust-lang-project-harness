//! Agent-facing selection advice for modular verification report artifacts.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::analysis::RustVerificationAnalysisProfile;
use super::report::{
    RustVerificationReportArtifact, RustVerificationReportArtifactRole,
    RustVerificationReportBundle, RustVerificationReportPersistence,
};

/// Structured artifact selection advice for windowed Agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportSelectionAdvice {
    /// Number of artifacts available in the source manifest.
    pub artifact_count: usize,
    /// First artifact an Agent should load, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<RustVerificationReportSelectionArtifact>,
    /// Stable reason for the selected first artifact.
    pub reason: RustVerificationReportSelectionReason,
    /// Scale facts used while selecting artifact order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<RustVerificationReportSelectionScale>,
    /// Full ordered artifact list, from lowest-window setup payload to heavier state.
    pub order: Vec<RustVerificationReportSelectionArtifact>,
}

/// Lightweight artifact reference used by selection advice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportSelectionArtifact {
    /// Stable report contract key.
    pub key: String,
    /// Agent-facing role for this artifact.
    pub role: RustVerificationReportArtifactRole,
    /// Recommended filename for persisted artifact payloads.
    pub artifact_name: String,
    /// Recommended persistence target from the manifest.
    pub persistence: RustVerificationReportPersistence,
}

/// Project scale facts that influence artifact selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportSelectionScale {
    /// Number of package scopes analyzed.
    pub package_count: usize,
    /// Number of parsed Rust files across package scopes.
    pub rust_file_count: usize,
    /// Number of parser-known source modules across package scopes.
    pub source_module_count: usize,
    /// Number of owner branches derived by the reasoning tree.
    pub owner_branch_count: usize,
    /// Number of Cargo dependency facts parsed for profile inference.
    pub cargo_dependency_count: usize,
}

/// Stable reason code for the selected first artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReportSelectionReason {
    /// There are no active report artifacts.
    NoActiveReportArtifacts,
    /// Load the analysis profile before selecting heavier payloads.
    LoadAnalysisProfileBeforePayloadSelection,
    /// The supplied profile shows a broad surface, so scope the window first.
    LargeAnalysisSurfaceScopeWindowFirst,
    /// Baseline evidence is the most actionable active payload.
    LoadBaselineEvidenceForActiveTasks,
    /// Skill dispatch index is more useful than prompt state for the next action.
    LoadSkillDispatchIndexBeforePromptState,
    /// Prompt state is needed for receipt and waiver matching.
    LoadPromptStateForReceiptMatching,
    /// Only one custom artifact is available.
    SingleCustomArtifact,
    /// Fall back to manifest order because no known role ranked higher.
    FallbackToManifestOrder,
}

impl RustVerificationReportSelectionReason {
    /// Return a stable lowercase reason code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveReportArtifacts => "no_active_report_artifacts",
            Self::LoadAnalysisProfileBeforePayloadSelection => {
                "load_analysis_profile_before_payload_selection"
            }
            Self::LargeAnalysisSurfaceScopeWindowFirst => {
                "large_analysis_surface_scope_window_first"
            }
            Self::LoadBaselineEvidenceForActiveTasks => "load_baseline_evidence_for_active_tasks",
            Self::LoadSkillDispatchIndexBeforePromptState => {
                "load_skill_dispatch_index_before_prompt_state"
            }
            Self::LoadPromptStateForReceiptMatching => "load_prompt_state_for_receipt_matching",
            Self::SingleCustomArtifact => "single_custom_artifact",
            Self::FallbackToManifestOrder => "fallback_to_manifest_order",
        }
    }
}

/// Build structured artifact selection advice for windowed Agents.
#[must_use]
pub fn build_rust_verification_report_selection_advice(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> RustVerificationReportSelectionAdvice {
    if bundle.is_empty() {
        return RustVerificationReportSelectionAdvice {
            artifact_count: 0,
            first: None,
            reason: RustVerificationReportSelectionReason::NoActiveReportArtifacts,
            scale: analysis_profile.map(RustVerificationReportSelectionScale::from),
            order: Vec::new(),
        };
    }

    let first = select_first_artifact(bundle, analysis_profile);
    let order = ordered_artifacts(bundle, first, analysis_profile);
    RustVerificationReportSelectionAdvice {
        artifact_count: bundle.artifacts.len(),
        first: Some(RustVerificationReportSelectionArtifact::from(first)),
        reason: selection_reason(first, bundle, analysis_profile),
        scale: analysis_profile.map(RustVerificationReportSelectionScale::from),
        order: order
            .into_iter()
            .map(RustVerificationReportSelectionArtifact::from)
            .collect(),
    }
}

/// Render structured artifact selection advice as JSON.
///
/// # Errors
///
/// Returns a serialization error if the advice cannot be encoded.
pub fn render_rust_verification_report_selection_advice_json(
    advice: &RustVerificationReportSelectionAdvice,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(advice)
}

/// Render a compact artifact reading order for windowed Agents.
///
/// The advice intentionally stays role-based. Callers can provide an analysis
/// profile when they already have one; otherwise the selector will prefer the
/// explicit `analysis_profile` artifact, when present, before heavier payloads.
#[must_use]
pub fn render_rust_verification_report_selection_advice(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> String {
    let advice = build_rust_verification_report_selection_advice(bundle, analysis_profile);
    render_rust_verification_report_selection_advice_compact(&advice)
}

fn render_rust_verification_report_selection_advice_compact(
    advice: &RustVerificationReportSelectionAdvice,
) -> String {
    let mut rendered = String::new();
    let Some(first) = advice.first.as_ref() else {
        let _ = write!(
            rendered,
            "[verify-report-select] artifacts=0 first=<none> reason={}",
            advice.reason.as_str()
        );
        return rendered;
    };
    let _ = writeln!(
        rendered,
        "[verify-report-select] artifacts={} first={} role={} reason={}",
        advice.artifact_count,
        first.key,
        first.role.as_str(),
        advice.reason.as_str()
    );
    if let Some(scale) = &advice.scale {
        render_scale_line(scale, &mut rendered);
    }
    render_order_line(&advice.order, &mut rendered);
    rendered.trim_end().to_string()
}

fn select_first_artifact<'a>(
    bundle: &'a RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> &'a RustVerificationReportArtifact {
    let should_scope_with_analysis =
        analysis_profile.is_none() || analysis_profile.is_some_and(is_large_analysis_profile);
    if let (true, Some(artifact)) = (
        should_scope_with_analysis,
        first_artifact_for_role(bundle, RustVerificationReportArtifactRole::AnalysisProfile),
    ) {
        return artifact;
    }
    for role in [
        RustVerificationReportArtifactRole::BaselineEvidence,
        RustVerificationReportArtifactRole::SkillDispatchIndex,
        RustVerificationReportArtifactRole::PromptState,
        RustVerificationReportArtifactRole::AnalysisProfile,
        RustVerificationReportArtifactRole::Custom,
    ] {
        if let Some(artifact) = first_artifact_for_role(bundle, role) {
            return artifact;
        }
    }
    bundle
        .artifacts
        .first()
        .expect("non-empty bundle must contain an artifact")
}

fn first_artifact_for_role(
    bundle: &RustVerificationReportBundle,
    role: RustVerificationReportArtifactRole,
) -> Option<&RustVerificationReportArtifact> {
    bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.role == role)
}

fn selection_reason(
    first: &RustVerificationReportArtifact,
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> RustVerificationReportSelectionReason {
    match first.role {
        RustVerificationReportArtifactRole::AnalysisProfile if analysis_profile.is_none() => {
            RustVerificationReportSelectionReason::LoadAnalysisProfileBeforePayloadSelection
        }
        RustVerificationReportArtifactRole::AnalysisProfile => {
            RustVerificationReportSelectionReason::LargeAnalysisSurfaceScopeWindowFirst
        }
        RustVerificationReportArtifactRole::BaselineEvidence => {
            RustVerificationReportSelectionReason::LoadBaselineEvidenceForActiveTasks
        }
        RustVerificationReportArtifactRole::SkillDispatchIndex => {
            RustVerificationReportSelectionReason::LoadSkillDispatchIndexBeforePromptState
        }
        RustVerificationReportArtifactRole::PromptState => {
            RustVerificationReportSelectionReason::LoadPromptStateForReceiptMatching
        }
        RustVerificationReportArtifactRole::Custom if bundle.artifacts.len() == 1 => {
            RustVerificationReportSelectionReason::SingleCustomArtifact
        }
        RustVerificationReportArtifactRole::Custom => {
            RustVerificationReportSelectionReason::FallbackToManifestOrder
        }
    }
}

fn render_scale_line(scale: &RustVerificationReportSelectionScale, rendered: &mut String) {
    let _ = writeln!(
        rendered,
        "   |scale: packages={} rust_files={} source_modules={} owner_branches={} cargo_dependencies={}",
        scale.package_count,
        scale.rust_file_count,
        scale.source_module_count,
        scale.owner_branch_count,
        scale.cargo_dependency_count
    );
}

fn ordered_artifacts<'a>(
    bundle: &'a RustVerificationReportBundle,
    first: &'a RustVerificationReportArtifact,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> Vec<&'a RustVerificationReportArtifact> {
    let mut ordered = vec![first];
    for role in remaining_role_order(first, analysis_profile) {
        for artifact in bundle.artifacts_for_role(role) {
            if !ordered.iter().any(|ordered| ordered.key == artifact.key) {
                ordered.push(artifact);
            }
        }
    }
    for artifact in &bundle.artifacts {
        if !ordered.iter().any(|ordered| ordered.key == artifact.key) {
            ordered.push(artifact);
        }
    }
    ordered
}

fn render_order_line(order: &[RustVerificationReportSelectionArtifact], rendered: &mut String) {
    let ordered_keys = order
        .iter()
        .map(|artifact| artifact.key.as_str())
        .collect::<Vec<_>>();
    let _ = write!(rendered, "   |order: {}", ordered_keys.join(" -> "));
}

fn remaining_role_order(
    first: &RustVerificationReportArtifact,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
) -> Vec<RustVerificationReportArtifactRole> {
    let mut roles = vec![
        RustVerificationReportArtifactRole::BaselineEvidence,
        RustVerificationReportArtifactRole::SkillDispatchIndex,
        RustVerificationReportArtifactRole::PromptState,
    ];
    if analysis_profile.is_none()
        || analysis_profile.is_some_and(is_large_analysis_profile)
        || first.role != RustVerificationReportArtifactRole::AnalysisProfile
    {
        roles.push(RustVerificationReportArtifactRole::AnalysisProfile);
    }
    roles.push(RustVerificationReportArtifactRole::Custom);
    roles
}

fn is_large_analysis_profile(profile: &RustVerificationAnalysisProfile) -> bool {
    profile.package_count > 1
        || profile.rust_file_count >= 50
        || profile.source_module_count >= 80
        || profile.owner_branch_count >= 20
        || profile.cargo_dependency_count >= 20
}

impl From<&RustVerificationReportArtifact> for RustVerificationReportSelectionArtifact {
    fn from(artifact: &RustVerificationReportArtifact) -> Self {
        Self {
            key: artifact.key.clone(),
            role: artifact.role,
            artifact_name: artifact.artifact_name.clone(),
            persistence: artifact.persistence,
        }
    }
}

impl From<&RustVerificationAnalysisProfile> for RustVerificationReportSelectionScale {
    fn from(profile: &RustVerificationAnalysisProfile) -> Self {
        Self {
            package_count: profile.package_count,
            rust_file_count: profile.rust_file_count,
            source_module_count: profile.source_module_count,
            owner_branch_count: profile.owner_branch_count,
            cargo_dependency_count: profile.cargo_dependency_count,
        }
    }
}
