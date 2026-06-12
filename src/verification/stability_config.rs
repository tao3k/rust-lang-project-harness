//! Project-owned stability picture configuration.

use serde::{Deserialize, Serialize};

/// Project-owned stability picture requirements for agent planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityPictureConfig {
    /// Require a long-running command and iteration window.
    pub require_long_running_simulation: bool,
    /// Require latency distribution or a project-owned performance interface.
    pub require_performance_interface: bool,
    /// Require resource-growth evidence such as RSS, file descriptors, or threads.
    pub require_resource_delta: bool,
    /// Require cache, queue, database, or artifact growth evidence.
    pub require_state_growth: bool,
    /// Require repeated-run determinism or a bounded nondeterminism statement.
    pub require_determinism: bool,
    /// Require a durable stability report artifact.
    pub require_stability_artifact: bool,
    /// Optional minimum long-run iteration count expected by this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_iterations: Option<u64>,
    /// Optional minimum long-run duration expected by this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_duration_seconds: Option<u64>,
}

impl Default for RustVerificationStabilityPictureConfig {
    fn default() -> Self {
        Self {
            require_long_running_simulation: true,
            require_performance_interface: true,
            require_resource_delta: true,
            require_state_growth: true,
            require_determinism: true,
            require_stability_artifact: true,
            min_iterations: None,
            min_duration_seconds: None,
        }
    }
}

impl RustVerificationStabilityPictureConfig {
    /// Build the default stability picture configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure whether long-running simulation evidence is required.
    #[must_use]
    pub const fn with_long_running_simulation_required(mut self, required: bool) -> Self {
        self.require_long_running_simulation = required;
        self
    }

    /// Configure whether performance-interface evidence is required.
    #[must_use]
    pub const fn with_performance_interface_required(mut self, required: bool) -> Self {
        self.require_performance_interface = required;
        self
    }

    /// Configure whether resource-growth evidence is required.
    #[must_use]
    pub const fn with_resource_delta_required(mut self, required: bool) -> Self {
        self.require_resource_delta = required;
        self
    }

    /// Configure whether state-growth evidence is required.
    #[must_use]
    pub const fn with_state_growth_required(mut self, required: bool) -> Self {
        self.require_state_growth = required;
        self
    }

    /// Configure whether determinism evidence is required.
    #[must_use]
    pub const fn with_determinism_required(mut self, required: bool) -> Self {
        self.require_determinism = required;
        self
    }

    /// Configure whether a durable stability artifact is required.
    #[must_use]
    pub const fn with_stability_artifact_required(mut self, required: bool) -> Self {
        self.require_stability_artifact = required;
        self
    }

    /// Configure the minimum expected long-run iteration count.
    #[must_use]
    pub const fn with_min_iterations(mut self, iterations: u64) -> Self {
        self.min_iterations = Some(iterations);
        self
    }

    /// Configure the minimum expected long-run duration in seconds.
    #[must_use]
    pub const fn with_min_duration_seconds(mut self, seconds: u64) -> Self {
        self.min_duration_seconds = Some(seconds);
        self
    }

    /// Return the receipt evidence keys required by this stability picture.
    #[must_use]
    pub fn required_receipt_evidence_keys(&self) -> Vec<&'static str> {
        let mut keys = Vec::new();
        if self.require_long_running_simulation {
            keys.extend(["stability_command", "iteration_window"]);
        }
        if self.require_performance_interface {
            keys.push("latency_distribution");
        }
        if self.require_resource_delta {
            keys.push("resource_delta");
        }
        if self.require_state_growth {
            keys.push("state_growth");
        }
        if self.require_determinism {
            keys.push("determinism");
        }
        if self.require_stability_artifact {
            keys.push("stability_artifact");
        }
        keys
    }

    /// Review this configuration for no-op or contradictory settings.
    #[must_use]
    pub fn review(&self) -> RustVerificationStabilityPictureConfigReview {
        let mut warnings = Vec::new();
        if self.required_receipt_evidence_keys().is_empty() {
            warnings.push(RustVerificationStabilityPictureConfigWarning::new(
                "no_required_axes",
                "stability picture has no required evidence axes",
            ));
        }
        if !self.require_long_running_simulation && self.min_iterations.is_some() {
            warnings.push(RustVerificationStabilityPictureConfigWarning::new(
                "min_iterations_without_long_run",
                "min_iterations is ignored unless long-running simulation is required",
            ));
        }
        if !self.require_long_running_simulation && self.min_duration_seconds.is_some() {
            warnings.push(RustVerificationStabilityPictureConfigWarning::new(
                "min_duration_without_long_run",
                "min_duration_seconds is ignored unless long-running simulation is required",
            ));
        }
        RustVerificationStabilityPictureConfigReview { warnings }
    }
}

/// Non-fatal stability picture configuration review result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityPictureConfigReview {
    /// Configuration warnings that should be surfaced to the agent.
    pub warnings: Vec<RustVerificationStabilityPictureConfigWarning>,
}

impl RustVerificationStabilityPictureConfigReview {
    /// Return whether the review found no warnings.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }
}

/// One non-fatal stability picture configuration warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityPictureConfigWarning {
    /// Stable warning key.
    pub key: String,
    /// Agent-readable warning message.
    pub message: String,
}

impl RustVerificationStabilityPictureConfigWarning {
    /// Build one warning.
    #[must_use]
    pub fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            message: message.into(),
        }
    }
}
