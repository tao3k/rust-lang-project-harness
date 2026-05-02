//! Modular verification report artifacts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::{
    RustVerificationPlan, RustVerificationReportObligation, RustVerificationTaskKind,
};
use super::performance::build_rust_verification_performance_index;
use super::task_index::build_rust_verification_task_index;

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

/// Runtime trace and time budget guidance for one report artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportTraceConfig {
    /// Stable trace profile label.
    pub profile: String,
    /// Suggested upper bound for producing this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_seconds: Option<u64>,
    /// Suggested sampling interval for profilers or benchmark monitors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_interval_ms: Option<u64>,
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
        self.max_seconds = Some(max_seconds);
        self
    }

    /// Attach a suggested trace sampling interval.
    #[must_use]
    pub const fn with_sample_interval_ms(mut self, sample_interval_ms: u64) -> Self {
        self.sample_interval_ms = Some(sample_interval_ms);
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
            default_trace: Some(
                RustVerificationReportTraceConfig::new("standard")
                    .with_max_seconds(60)
                    .with_sample_interval_ms(1000),
            ),
            artifact_traces: BTreeMap::from([(
                "performance_index_json".to_string(),
                RustVerificationReportTraceConfig::new("performance")
                    .with_max_seconds(300)
                    .with_sample_interval_ms(250)
                    .with_raw_traces(),
            )]),
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
            ]),
        }
    }
}

impl RustVerificationReportOptions {
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

impl RustVerificationReportArtifact {
    /// Build one report manifest entry from an obligation.
    #[must_use]
    pub fn from_obligation(
        obligation: &RustVerificationReportObligation,
        options: &RustVerificationReportOptions,
    ) -> Self {
        Self {
            key: obligation.key.clone(),
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
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Persistable report artifacts requested by the active verification plan.
    pub artifacts: Vec<RustVerificationReportArtifact>,
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

    fn with_artifacts(&self, artifacts: Vec<RustVerificationReportArtifact>) -> Self {
        Self {
            project_root: self.project_root.clone(),
            artifacts,
        }
    }
}

/// Filesystem layout used to persist modular verification reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustVerificationReportWriteConfig {
    /// Project root whose absolute path may appear in rendered artifacts.
    pub project_root: PathBuf,
    /// Source-controlled directory for compact baseline artifacts.
    pub source_baseline_dir: PathBuf,
    /// Runtime cache directory for verbose or machine-local artifacts.
    pub runtime_cache_dir: PathBuf,
    /// Stable placeholder used when compacting `project_root` in JSON output.
    pub project_root_placeholder: String,
}

impl RustVerificationReportWriteConfig {
    /// Build a report write config.
    #[must_use]
    pub fn new(
        project_root: impl Into<PathBuf>,
        source_baseline_dir: impl Into<PathBuf>,
        runtime_cache_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            source_baseline_dir: source_baseline_dir.into(),
            runtime_cache_dir: runtime_cache_dir.into(),
            project_root_placeholder: "$CRATE_ROOT".to_string(),
        }
    }

    /// Override the placeholder used for project-root path compaction.
    #[must_use]
    pub fn with_project_root_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.project_root_placeholder = placeholder.into();
        self
    }
}

/// Paths written by `write_rust_verification_reports`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustVerificationReportWriteReceipt {
    /// Files written under the source baseline directory.
    pub source_baseline_paths: Vec<PathBuf>,
    /// Files written under the runtime cache directory.
    pub runtime_cache_paths: Vec<PathBuf>,
}

/// Error raised while writing modular verification reports.
#[derive(Debug)]
pub enum RustVerificationReportWriteError {
    /// A report artifact could not be serialized.
    Json(serde_json::Error),
    /// A filesystem operation failed.
    Io {
        /// Path being created or written when the error occurred.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

impl fmt::Display for RustVerificationReportWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to render verification report: {error}"),
            Self::Io { path, source } => write!(
                formatter,
                "failed to write verification report at {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RustVerificationReportWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<serde_json::Error> for RustVerificationReportWriteError {
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
    RustVerificationReportBundle {
        project_root: plan.project_root.clone(),
        artifacts: plan
            .report_obligations
            .iter()
            .map(|obligation| RustVerificationReportArtifact::from_obligation(obligation, options))
            .collect(),
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

/// Write modular verification report artifacts using their persistence policy.
///
/// The source baseline manifest contains only source-controlled artifacts. The
/// runtime cache manifest contains the full bundle so local tooling can inspect
/// both source and cache report responsibilities from one machine-local file.
///
/// # Errors
///
/// Returns an error if directories cannot be created, artifacts cannot be
/// serialized, or files cannot be written.
pub fn write_rust_verification_reports(
    plan: &RustVerificationPlan,
    config: &RustVerificationReportWriteConfig,
) -> Result<RustVerificationReportWriteReceipt, RustVerificationReportWriteError> {
    create_dir_all(&config.source_baseline_dir)?;
    create_dir_all(&config.runtime_cache_dir)?;

    let bundle = build_rust_verification_report_bundle(plan);
    let source_artifacts: Vec<_> = bundle
        .source_baseline_artifacts()
        .into_iter()
        .cloned()
        .collect();
    let cache_artifacts: Vec<_> = bundle
        .runtime_cache_artifacts()
        .into_iter()
        .cloned()
        .collect();

    let mut receipt = RustVerificationReportWriteReceipt::default();
    let source_bundle = bundle.with_artifacts(source_artifacts.clone());
    let source_manifest = config
        .source_baseline_dir
        .join("verification_report_manifest.json");
    write_json(
        &source_manifest,
        &compact_project_root(
            &serde_json::to_string(&source_bundle)?,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    receipt.source_baseline_paths.push(source_manifest);

    let cache_manifest = config
        .runtime_cache_dir
        .join("verification_report_manifest.json");
    write_json(
        &cache_manifest,
        &compact_project_root(
            &serde_json::to_string(&bundle)?,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    receipt.runtime_cache_paths.push(cache_manifest);

    for artifact in source_artifacts {
        write_artifact(
            plan,
            &artifact,
            &config.source_baseline_dir,
            config,
            |path| {
                receipt.source_baseline_paths.push(path);
            },
        )?;
    }
    for artifact in cache_artifacts {
        write_artifact(plan, &artifact, &config.runtime_cache_dir, config, |path| {
            receipt.runtime_cache_paths.push(path);
        })?;
    }

    Ok(receipt)
}

fn write_artifact(
    plan: &RustVerificationPlan,
    artifact: &RustVerificationReportArtifact,
    directory: &Path,
    config: &RustVerificationReportWriteConfig,
    mut record_path: impl FnMut(PathBuf),
) -> Result<(), RustVerificationReportWriteError> {
    let Some(payload) = render_rust_verification_report_artifact_json(plan, &artifact.key)? else {
        return Ok(());
    };
    let path = directory.join(&artifact.artifact_name);
    write_json(
        &path,
        &compact_project_root(
            &payload,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    record_path(path);
    Ok(())
}

fn create_dir_all(path: &Path) -> Result<(), RustVerificationReportWriteError> {
    std::fs::create_dir_all(path).map_err(|source| RustVerificationReportWriteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json(path: &Path, payload: &str) -> Result<(), RustVerificationReportWriteError> {
    std::fs::write(path, payload).map_err(|source| RustVerificationReportWriteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn compact_project_root(payload: &str, project_root: &Path, placeholder: &str) -> String {
    let root = project_root.to_string_lossy();
    payload.replace(root.as_ref(), placeholder)
}

fn path_buf_is_empty(path: &std::path::Path) -> bool {
    path.as_os_str().is_empty()
}
