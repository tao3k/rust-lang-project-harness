//! Modular verification report artifacts.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::model::{
    RustVerificationPlan, RustVerificationReportObligation, RustVerificationTaskKind,
};
use super::report_manifest::RustVerificationReportManifestSchema;
use super::report_options::{
    ANALYSIS_PROFILE_ARTIFACT_KEY, SELECTION_ADVICE_SIDECAR_KEY, STABILITY_PICTURE_ARTIFACT_KEY,
};
pub use super::report_options::{
    RustVerificationReportArtifactRole, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportSidecarRole,
    RustVerificationReportTemplate, RustVerificationReportTraceConfig,
};

/// Manifest entry for one persistable verification report artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportArtifact {
    /// Stable report contract key.
    pub key: String,
    /// Agent-facing role for selecting this artifact from a manifest.
    pub role: RustVerificationReportArtifactRole,
    /// Recommended artifact filename for embedding projects.
    pub artifact_name: String,
    /// Harness renderer or index builder that produces the artifact payload.
    pub renderer: String,
    /// Why this artifact should be persisted for later comparison.
    pub reason: String,
    /// Active task families covered by this artifact.
    pub task_kinds: BTreeSet<RustVerificationTaskKind>,
    /// Active task fingerprints covered by this artifact.
    pub task_fingerprints: Vec<String>,
    /// Where this artifact should be persisted by default.
    pub persistence: RustVerificationReportPersistence,
    /// Small template contract for this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<RustVerificationReportTemplate>,
    /// Runtime trace and time budget guidance for this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<RustVerificationReportTraceConfig>,
}

/// Manifest entry for a machine-local report sidecar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportSidecar {
    /// Stable sidecar contract key.
    pub key: String,
    /// Agent-facing role for discovering this sidecar.
    pub role: RustVerificationReportSidecarRole,
    /// Recommended sidecar filename for embedding projects.
    pub artifact_name: String,
    /// Harness renderer that produces the sidecar payload.
    pub renderer: String,
    /// Why this sidecar should be persisted for local Agent access.
    pub reason: String,
    /// Where this sidecar should be persisted by default.
    pub persistence: RustVerificationReportPersistence,
}

impl RustVerificationReportArtifact {
    /// Build one report manifest entry from an obligation.
    #[must_use]
    pub fn from_obligation(
        obligation: &RustVerificationReportObligation,
        options: &RustVerificationReportOptions,
    ) -> Self {
        Self {
            key: obligation.key.clone(),
            role: report_artifact_role(&obligation.key),
            artifact_name: obligation.suggested_artifact_name.clone(),
            renderer: obligation.renderer.clone(),
            reason: obligation.reason.clone(),
            task_kinds: obligation.task_kinds.clone(),
            task_fingerprints: obligation.task_fingerprints.clone(),
            persistence: options
                .artifact_persistence
                .get(&obligation.key)
                .copied()
                .unwrap_or(RustVerificationReportPersistence::RuntimeCache),
            template: options.artifact_templates.get(&obligation.key).cloned(),
            trace: options
                .artifact_traces
                .get(&obligation.key)
                .cloned()
                .or_else(|| options.default_trace.clone()),
        }
    }

    /// Number of active tasks covered by this artifact.
    #[must_use]
    pub fn task_count(&self) -> usize {
        self.task_fingerprints.len()
    }
}

/// Small report manifest for all active modular report artifacts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportBundle {
    /// Manifest schema metadata for downstream compatibility checks.
    pub schema: RustVerificationReportManifestSchema,
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Persistable report artifacts requested by the active verification plan.
    pub artifacts: Vec<RustVerificationReportArtifact>,
    /// Machine-local sidecars that help Agents discover or select artifacts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<RustVerificationReportSidecar>,
}

impl RustVerificationReportBundle {
    /// Return whether no report artifact is required.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Return one report artifact by contract key.
    #[must_use]
    pub fn artifact(&self, key: &str) -> Option<&RustVerificationReportArtifact> {
        self.artifacts.iter().find(|artifact| artifact.key == key)
    }

    /// Return artifacts recommended for source-controlled baselines.
    #[must_use]
    pub fn source_baseline_artifacts(&self) -> Vec<&RustVerificationReportArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| {
                artifact.persistence == RustVerificationReportPersistence::SourceBaseline
            })
            .collect()
    }

    /// Return artifacts recommended for runtime cache.
    #[must_use]
    pub fn runtime_cache_artifacts(&self) -> Vec<&RustVerificationReportArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| {
                artifact.persistence == RustVerificationReportPersistence::RuntimeCache
            })
            .collect()
    }

    /// Return sidecars recommended for source-controlled baselines.
    #[must_use]
    pub fn source_baseline_sidecars(&self) -> Vec<&RustVerificationReportSidecar> {
        self.sidecars
            .iter()
            .filter(|sidecar| {
                sidecar.persistence == RustVerificationReportPersistence::SourceBaseline
            })
            .collect()
    }

    /// Return sidecars recommended for runtime cache.
    #[must_use]
    pub fn runtime_cache_sidecars(&self) -> Vec<&RustVerificationReportSidecar> {
        self.sidecars
            .iter()
            .filter(|sidecar| {
                sidecar.persistence == RustVerificationReportPersistence::RuntimeCache
            })
            .collect()
    }

    /// Return artifacts that match one Agent-facing selection role.
    #[must_use]
    pub fn artifacts_for_role(
        &self,
        role: RustVerificationReportArtifactRole,
    ) -> Vec<&RustVerificationReportArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.role == role)
            .collect()
    }

    /// Return one report sidecar by contract key.
    #[must_use]
    pub fn sidecar(&self, key: &str) -> Option<&RustVerificationReportSidecar> {
        self.sidecars.iter().find(|sidecar| sidecar.key == key)
    }

    /// Return sidecars that match one Agent-facing selection role.
    #[must_use]
    pub fn sidecars_for_role(
        &self,
        role: RustVerificationReportSidecarRole,
    ) -> Vec<&RustVerificationReportSidecar> {
        self.sidecars
            .iter()
            .filter(|sidecar| sidecar.role == role)
            .collect()
    }

    pub(super) fn with_parts(
        &self,
        artifacts: Vec<RustVerificationReportArtifact>,
        sidecars: Vec<RustVerificationReportSidecar>,
    ) -> Self {
        Self {
            schema: self.schema.clone(),
            project_root: self.project_root.clone(),
            artifacts,
            sidecars,
        }
    }
}

/// Build the small manifest for all durable reports required by a plan.
#[must_use]
pub fn build_rust_verification_report_bundle(
    plan: &RustVerificationPlan,
) -> RustVerificationReportBundle {
    build_rust_verification_report_bundle_with_options(
        plan,
        &RustVerificationReportOptions::default(),
    )
}

/// Build the small manifest with configurable trace and template guidance.
#[must_use]
pub fn build_rust_verification_report_bundle_with_options(
    plan: &RustVerificationPlan,
    options: &RustVerificationReportOptions,
) -> RustVerificationReportBundle {
    let mut artifacts = plan
        .report_obligations
        .iter()
        .map(|obligation| RustVerificationReportArtifact::from_obligation(obligation, options))
        .collect::<Vec<_>>();
    if options.include_analysis_profile {
        artifacts.push(analysis_profile_artifact(options));
    }
    let sidecars = if options.include_selection_advice {
        vec![selection_advice_sidecar()]
    } else {
        Vec::new()
    };
    RustVerificationReportBundle {
        schema: RustVerificationReportManifestSchema::default(),
        project_root: plan.project_root.clone(),
        artifacts,
        sidecars,
    }
}

/// Render the small modular verification report manifest as JSON.
///
/// # Errors
///
/// Returns a serialization error if the manifest cannot be encoded as JSON.
pub fn render_rust_verification_report_bundle_json(
    plan: &RustVerificationPlan,
) -> Result<String, serde_json::Error> {
    let bundle = build_rust_verification_report_bundle(plan);
    serde_json::to_string(&bundle)
}

/// Render a compact report manifest for Agent artifact selection.
#[must_use]
pub fn render_rust_verification_report_bundle(bundle: &RustVerificationReportBundle) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "[verify-report-bundle] artifacts={} sidecars={} source_baseline={} runtime_cache={} schema={}",
        bundle.artifacts.len(),
        bundle.sidecars.len(),
        bundle.source_baseline_artifacts().len(),
        bundle.runtime_cache_artifacts().len(),
        bundle.schema.compact_label()
    );
    for artifact in &bundle.artifacts {
        render_report_bundle_artifact(artifact, &mut rendered);
    }
    for sidecar in &bundle.sidecars {
        render_report_bundle_sidecar(sidecar, &mut rendered);
    }
    rendered.trim_end().to_string()
}

fn path_buf_is_empty(path: &std::path::Path) -> bool {
    path.as_os_str().is_empty()
}

fn render_report_bundle_artifact(artifact: &RustVerificationReportArtifact, rendered: &mut String) {
    let _ = write!(
        rendered,
        "   |artifact: role={} key={} persistence={} file={} tasks={}",
        artifact.role.as_str(),
        artifact.key,
        artifact.persistence.as_str(),
        artifact.artifact_name,
        artifact.task_count()
    );
    if let Some(trace) = &artifact.trace {
        let _ = write!(rendered, " trace={}", trace.profile);
        if let Some(max_seconds) = trace.max_seconds {
            let _ = write!(rendered, " max_s={}", max_seconds.as_u64());
        }
        if let Some(sample_interval_ms) = trace.sample_interval_ms {
            let _ = write!(rendered, " sample_ms={}", sample_interval_ms.as_u64());
        }
        if trace.include_raw_traces {
            let _ = write!(rendered, " raw=true");
        }
    }
    if let Some(template) = &artifact.template {
        let _ = write!(rendered, " template={}", template.template_id);
    }
    let _ = writeln!(rendered);
    let _ = writeln!(
        rendered,
        "   |renderer: {}={}",
        artifact.key, artifact.renderer
    );
}

fn render_report_bundle_sidecar(sidecar: &RustVerificationReportSidecar, rendered: &mut String) {
    let _ = writeln!(
        rendered,
        "   |sidecar: role={} key={} persistence={} file={}",
        sidecar.role.as_str(),
        sidecar.key,
        sidecar.persistence.as_str(),
        sidecar.artifact_name
    );
    let _ = writeln!(
        rendered,
        "   |renderer: {}={}",
        sidecar.key, sidecar.renderer
    );
}

fn analysis_profile_artifact(
    options: &RustVerificationReportOptions,
) -> RustVerificationReportArtifact {
    RustVerificationReportArtifact {
        key: ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
        role: RustVerificationReportArtifactRole::AnalysisProfile,
        artifact_name: "analysis_profile.json".to_string(),
        renderer: "build_rust_verification_analysis_profile_with_config + render_rust_verification_analysis_profile_json".to_string(),
        reason: "persist parser analysis scale profile for Agent planning and optimization passes"
            .to_string(),
        task_kinds: BTreeSet::new(),
        task_fingerprints: Vec::new(),
        persistence: options
            .artifact_persistence
            .get(ANALYSIS_PROFILE_ARTIFACT_KEY)
            .copied()
            .unwrap_or(RustVerificationReportPersistence::RuntimeCache),
        template: options
            .artifact_templates
            .get(ANALYSIS_PROFILE_ARTIFACT_KEY)
            .cloned(),
        trace: options
            .artifact_traces
            .get(ANALYSIS_PROFILE_ARTIFACT_KEY)
            .cloned()
            .or_else(|| options.default_trace.clone()),
    }
}

fn selection_advice_sidecar() -> RustVerificationReportSidecar {
    RustVerificationReportSidecar {
        key: SELECTION_ADVICE_SIDECAR_KEY.to_string(),
        role: RustVerificationReportSidecarRole::SelectionAdvice,
        artifact_name: "selection_advice.json".to_string(),
        renderer: "build_rust_verification_report_selection_advice + render_rust_verification_report_selection_advice_json".to_string(),
        reason: "persist Agent report reading order next to the runtime manifest".to_string(),
        persistence: RustVerificationReportPersistence::RuntimeCache,
    }
}

fn report_artifact_role(key: &str) -> RustVerificationReportArtifactRole {
    match key {
        "verification_plan_json" => RustVerificationReportArtifactRole::PromptState,
        "task_index_json" => RustVerificationReportArtifactRole::SkillDispatchIndex,
        "performance_index_json" => RustVerificationReportArtifactRole::BaselineEvidence,
        "stability_index_json" => RustVerificationReportArtifactRole::BaselineEvidence,
        STABILITY_PICTURE_ARTIFACT_KEY => RustVerificationReportArtifactRole::PromptState,
        ANALYSIS_PROFILE_ARTIFACT_KEY => RustVerificationReportArtifactRole::AnalysisProfile,
        _ => RustVerificationReportArtifactRole::Custom,
    }
}
