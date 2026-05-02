//! Modular verification report artifacts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::model::{
    RustVerificationPlan, RustVerificationReportObligation, RustVerificationTaskKind,
};
use super::performance::build_rust_verification_performance_index;

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

fn path_buf_is_empty(path: &std::path::Path) -> bool {
    path.as_os_str().is_empty()
}
