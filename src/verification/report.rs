//! Modular verification report artifacts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::RustHarnessConfig;

use super::analysis::{
    build_rust_verification_analysis_profile_with_config,
    render_rust_verification_analysis_profile_json,
};
use super::model::{
    RustVerificationPlan, RustVerificationReportObligation, RustVerificationTaskKind,
};
use super::performance::build_rust_verification_performance_index;
use super::report_manifest::RustVerificationReportManifestSchema;
use super::task_index::build_rust_verification_task_index;

const ANALYSIS_PROFILE_ARTIFACT_KEY: &str = "analysis_profile_json";
const SELECTION_ADVICE_SIDECAR_KEY: &str = "selection_advice_json";

/// Recommended persistence target for one report artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReportPersistence {
    /// Keep the artifact in runtime cache because it is verbose or machine-local.
    RuntimeCache,
    /// Commit the artifact as source-controlled baseline evidence.
    SourceBaseline,
}

impl RustVerificationReportPersistence {
    /// Return a stable lowercase persistence label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeCache => "runtime_cache",
            Self::SourceBaseline => "source_baseline",
        }
    }
}

/// Agent-facing role for selecting one report artifact from a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReportArtifactRole {
    /// Full verification plan state for receipt, waiver, and drift matching.
    PromptState,
    /// Compact configured-skill task dispatch index.
    SkillDispatchIndex,
    /// Durable performance evidence and baseline comparison index.
    BaselineEvidence,
    /// Project-scale parser analysis profile for planning and optimization.
    AnalysisProfile,
    /// Caller-defined artifact not recognized by the upstream role table.
    Custom,
}

impl RustVerificationReportArtifactRole {
    /// Return a stable lowercase role label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PromptState => "prompt_state",
            Self::SkillDispatchIndex => "skill_dispatch_index",
            Self::BaselineEvidence => "baseline_evidence",
            Self::AnalysisProfile => "analysis_profile",
            Self::Custom => "custom",
        }
    }
}

/// Agent-facing role for a report sidecar entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationReportSidecarRole {
    /// Structured reading-order advice for windowed Agents.
    SelectionAdvice,
    /// Caller-defined sidecar not recognized by the upstream role table.
    Custom,
}

impl RustVerificationReportSidecarRole {
    /// Return a stable lowercase sidecar role label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelectionAdvice => "selection_advice",
            Self::Custom => "custom",
        }
    }
}

/// Suggested upper bound in seconds for producing a verification artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationTraceMaxSeconds(u64);

impl RustVerificationTraceMaxSeconds {
    /// Build a trace runtime budget.
    #[must_use]
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Return the raw second count for renderers and tests.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Suggested sampling interval in milliseconds for trace monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationTraceSampleIntervalMs(u64);

impl RustVerificationTraceSampleIntervalMs {
    /// Build a trace sampling interval.
    #[must_use]
    pub const fn new(milliseconds: u64) -> Self {
        Self(milliseconds)
    }

    /// Return the raw millisecond count for renderers and tests.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Runtime trace and time budget guidance for one report artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportTraceConfig {
    /// Stable trace profile label.
    pub profile: String,
    /// Suggested upper bound for producing this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_seconds: Option<RustVerificationTraceMaxSeconds>,
    /// Suggested sampling interval for profilers or benchmark monitors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_interval_ms: Option<RustVerificationTraceSampleIntervalMs>,
    /// Whether the embedding project should preserve raw trace attachments.
    pub include_raw_traces: bool,
}

impl RustVerificationReportTraceConfig {
    /// Build a trace config with a stable profile label.
    #[must_use]
    pub fn new(profile: impl Into<String>) -> Self {
        Self {
            profile: profile.into(),
            max_seconds: None,
            sample_interval_ms: None,
            include_raw_traces: false,
        }
    }

    /// Attach a suggested runtime budget.
    #[must_use]
    pub const fn with_max_seconds(mut self, max_seconds: u64) -> Self {
        self.max_seconds = Some(RustVerificationTraceMaxSeconds::new(max_seconds));
        self
    }

    /// Attach a suggested trace sampling interval.
    #[must_use]
    pub const fn with_sample_interval_ms(mut self, sample_interval_ms: u64) -> Self {
        self.sample_interval_ms = Some(RustVerificationTraceSampleIntervalMs::new(
            sample_interval_ms,
        ));
        self
    }

    /// Request raw trace attachments.
    #[must_use]
    pub const fn with_raw_traces(mut self) -> Self {
        self.include_raw_traces = true;
        self
    }
}

/// Small template contract for a modular verification report artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportTemplate {
    /// Stable template id.
    pub template_id: String,
    /// Template schema version.
    pub schema_version: String,
    /// Sections an Agent should preserve when writing the artifact.
    pub required_sections: Vec<String>,
}

impl RustVerificationReportTemplate {
    /// Build a report template contract.
    #[must_use]
    pub fn new<I, S>(
        template_id: impl Into<String>,
        schema_version: impl Into<String>,
        required_sections: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            template_id: template_id.into(),
            schema_version: schema_version.into(),
            required_sections: required_sections.into_iter().map(Into::into).collect(),
        }
    }
}

/// Configurable report manifest options supplied by the embedding project or Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportOptions {
    /// Include the project-scale analysis profile as an explicit runtime artifact.
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_analysis_profile: bool,
    /// Include a runtime-cache selection advice sidecar when writing reports.
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_selection_advice: bool,
    /// Default trace config used when an artifact has no more specific config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_trace: Option<RustVerificationReportTraceConfig>,
    /// Per-artifact trace config keyed by report contract key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifact_traces: BTreeMap<String, RustVerificationReportTraceConfig>,
    /// Per-artifact template config keyed by report contract key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifact_templates: BTreeMap<String, RustVerificationReportTemplate>,
    /// Per-artifact persistence target keyed by report contract key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifact_persistence: BTreeMap<String, RustVerificationReportPersistence>,
}

impl Default for RustVerificationReportOptions {
    fn default() -> Self {
        Self {
            include_analysis_profile: false,
            include_selection_advice: false,
            default_trace: Some(
                RustVerificationReportTraceConfig::new("standard")
                    .with_max_seconds(60)
                    .with_sample_interval_ms(1000),
            ),
            artifact_traces: BTreeMap::from([
                (
                    "performance_index_json".to_string(),
                    RustVerificationReportTraceConfig::new("performance")
                        .with_max_seconds(300)
                        .with_sample_interval_ms(250)
                        .with_raw_traces(),
                ),
                (
                    ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
                    RustVerificationReportTraceConfig::new("analysis")
                        .with_max_seconds(60)
                        .with_sample_interval_ms(1000),
                ),
            ]),
            artifact_templates: BTreeMap::from([
                (
                    "verification_plan_json".to_string(),
                    RustVerificationReportTemplate::new(
                        "verification-plan",
                        "1",
                        ["tasks", "obligations", "receipts", "waivers"],
                    ),
                ),
                (
                    "task_index_json".to_string(),
                    RustVerificationReportTemplate::new(
                        "verification-task-index",
                        "1",
                        [
                            "kind",
                            "state",
                            "skill",
                            "required_evidence_keys",
                            "task_evidence",
                        ],
                    ),
                ),
                (
                    "performance_index_json".to_string(),
                    RustVerificationReportTemplate::new(
                        "performance-index",
                        "1",
                        [
                            "benchmark_command",
                            "baseline",
                            "regression_threshold",
                            "latency_or_throughput",
                            "profile_artifact",
                        ],
                    ),
                ),
                (
                    ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
                    RustVerificationReportTemplate::new(
                        "verification-analysis-profile",
                        "1",
                        [
                            "package_count",
                            "rust_file_count",
                            "source_module_count",
                            "owner_branch_count",
                            "cargo_dependency_count",
                            "packages",
                        ],
                    ),
                ),
            ]),
            artifact_persistence: BTreeMap::from([
                (
                    "verification_plan_json".to_string(),
                    RustVerificationReportPersistence::RuntimeCache,
                ),
                (
                    "task_index_json".to_string(),
                    RustVerificationReportPersistence::SourceBaseline,
                ),
                (
                    "performance_index_json".to_string(),
                    RustVerificationReportPersistence::SourceBaseline,
                ),
                (
                    ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
                    RustVerificationReportPersistence::RuntimeCache,
                ),
            ]),
        }
    }
}

impl RustVerificationReportOptions {
    /// Include the analysis profile as an explicit runtime-cache artifact.
    #[must_use]
    pub const fn with_analysis_profile_artifact(mut self) -> Self {
        self.include_analysis_profile = true;
        self
    }

    /// Include a runtime-cache artifact selection sidecar when reports are written.
    #[must_use]
    pub const fn with_selection_advice_sidecar(mut self) -> Self {
        self.include_selection_advice = true;
        self
    }

    /// Return options without default trace guidance.
    #[must_use]
    pub fn without_default_trace(mut self) -> Self {
        self.default_trace = None;
        self
    }

    /// Override the default trace guidance.
    #[must_use]
    pub fn with_default_trace(mut self, trace: RustVerificationReportTraceConfig) -> Self {
        self.default_trace = Some(trace);
        self
    }

    /// Override trace guidance for one artifact key.
    #[must_use]
    pub fn with_artifact_trace(
        mut self,
        key: impl Into<String>,
        trace: RustVerificationReportTraceConfig,
    ) -> Self {
        self.artifact_traces.insert(key.into(), trace);
        self
    }

    /// Override template guidance for one artifact key.
    #[must_use]
    pub fn with_artifact_template(
        mut self,
        key: impl Into<String>,
        template: RustVerificationReportTemplate,
    ) -> Self {
        self.artifact_templates.insert(key.into(), template);
        self
    }

    /// Override the persistence target for one artifact key.
    #[must_use]
    pub fn with_artifact_persistence(
        mut self,
        key: impl Into<String>,
        persistence: RustVerificationReportPersistence,
    ) -> Self {
        self.artifact_persistence.insert(key.into(), persistence);
        self
    }
}

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

/// Error raised while rendering one modular verification report artifact.
#[derive(Debug)]
pub enum RustVerificationReportArtifactRenderError {
    /// A report artifact could not be serialized.
    Json(serde_json::Error),
    /// Analysis profile construction failed before serialization.
    Analysis(String),
}

impl fmt::Display for RustVerificationReportArtifactRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Analysis(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RustVerificationReportArtifactRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Analysis(_) => None,
        }
    }
}

impl From<serde_json::Error> for RustVerificationReportArtifactRenderError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
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

/// Render one modular verification report artifact by contract key.
///
/// # Errors
///
/// Returns a serialization error if the selected artifact cannot be encoded as
/// JSON.
pub fn render_rust_verification_report_artifact_json(
    plan: &RustVerificationPlan,
    key: &str,
) -> Result<Option<String>, serde_json::Error> {
    match key {
        "verification_plan_json" => serde_json::to_string(plan).map(Some),
        "task_index_json" => {
            serde_json::to_string(&build_rust_verification_task_index(plan)).map(Some)
        }
        "performance_index_json" => {
            serde_json::to_string(&build_rust_verification_performance_index(plan)).map(Some)
        }
        _ => Ok(None),
    }
}

/// Render one modular verification report artifact with access to harness config.
///
/// # Errors
///
/// Returns an error if the selected artifact cannot be produced or serialized.
pub fn render_rust_verification_report_artifact_json_with_config(
    plan: &RustVerificationPlan,
    harness_config: &RustHarnessConfig,
    key: &str,
) -> Result<Option<String>, RustVerificationReportArtifactRenderError> {
    if key == ANALYSIS_PROFILE_ARTIFACT_KEY {
        let profile = build_rust_verification_analysis_profile_with_config(
            &plan.project_root,
            harness_config,
        )
        .map_err(RustVerificationReportArtifactRenderError::Analysis)?;
        return render_rust_verification_analysis_profile_json(&profile)
            .map(Some)
            .map_err(Into::into);
    }
    render_rust_verification_report_artifact_json(plan, key).map_err(Into::into)
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

fn is_false(value: &bool) -> bool {
    !value
}

fn report_artifact_role(key: &str) -> RustVerificationReportArtifactRole {
    match key {
        "verification_plan_json" => RustVerificationReportArtifactRole::PromptState,
        "task_index_json" => RustVerificationReportArtifactRole::SkillDispatchIndex,
        "performance_index_json" => RustVerificationReportArtifactRole::BaselineEvidence,
        ANALYSIS_PROFILE_ARTIFACT_KEY => RustVerificationReportArtifactRole::AnalysisProfile,
        _ => RustVerificationReportArtifactRole::Custom,
    }
}
