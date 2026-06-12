//! Verification report option and artifact role contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub(super) const ANALYSIS_PROFILE_ARTIFACT_KEY: &str = "analysis_profile_json";
pub(super) const SELECTION_ADVICE_SIDECAR_KEY: &str = "selection_advice_json";
pub(super) const STABILITY_PICTURE_ARTIFACT_KEY: &str = "stability_picture_json";

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
    /// Durable performance or stability baseline evidence index.
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
            artifact_traces: default_artifact_traces(),
            artifact_templates: default_artifact_templates(),
            artifact_persistence: default_artifact_persistence(),
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

fn default_artifact_traces() -> BTreeMap<String, RustVerificationReportTraceConfig> {
    BTreeMap::from([
        (
            "performance_index_json".to_string(),
            RustVerificationReportTraceConfig::new("performance")
                .with_max_seconds(300)
                .with_sample_interval_ms(250)
                .with_raw_traces(),
        ),
        (
            "stability_index_json".to_string(),
            RustVerificationReportTraceConfig::new("stability")
                .with_max_seconds(900)
                .with_sample_interval_ms(1000)
                .with_raw_traces(),
        ),
        (
            STABILITY_PICTURE_ARTIFACT_KEY.to_string(),
            RustVerificationReportTraceConfig::new("stability-picture")
                .with_max_seconds(60)
                .with_sample_interval_ms(1000),
        ),
        (
            ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
            RustVerificationReportTraceConfig::new("analysis")
                .with_max_seconds(60)
                .with_sample_interval_ms(1000),
        ),
    ])
}

fn default_artifact_templates() -> BTreeMap<String, RustVerificationReportTemplate> {
    BTreeMap::from([
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
            "stability_index_json".to_string(),
            RustVerificationReportTemplate::new(
                "stability-index",
                "1",
                [
                    "stability_command",
                    "iteration_window",
                    "latency_distribution",
                    "resource_delta",
                    "state_growth",
                    "determinism",
                    "stability_artifact",
                ],
            ),
        ),
        (
            STABILITY_PICTURE_ARTIFACT_KEY.to_string(),
            RustVerificationReportTemplate::new(
                "stability-picture",
                "1",
                [
                    "configured_axes",
                    "owner_overrides",
                    "api_path_overrides",
                    "missing_evidence",
                    "next_actions",
                    "config_warnings",
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
    ])
}

fn default_artifact_persistence() -> BTreeMap<String, RustVerificationReportPersistence> {
    BTreeMap::from([
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
            "stability_index_json".to_string(),
            RustVerificationReportPersistence::SourceBaseline,
        ),
        (
            STABILITY_PICTURE_ARTIFACT_KEY.to_string(),
            RustVerificationReportPersistence::RuntimeCache,
        ),
        (
            ANALYSIS_PROFILE_ARTIFACT_KEY.to_string(),
            RustVerificationReportPersistence::RuntimeCache,
        ),
    ])
}

fn is_false(value: &bool) -> bool {
    !value
}
