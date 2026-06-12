//! Stability fixture and baseline delta primitives.

use serde::{Deserialize, Serialize};

/// Stability fixture iteration count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationStabilityIterationCount(u64);

impl RustVerificationStabilityIterationCount {
    /// Build an iteration count.
    #[must_use]
    pub const fn new(iterations: u64) -> Self {
        Self(iterations)
    }

    /// Return the raw iteration count.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Stability fixture duration in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationStabilityDurationSeconds(u64);

impl RustVerificationStabilityDurationSeconds {
    /// Build a duration.
    #[must_use]
    pub const fn new(seconds: u64) -> Self {
        Self(seconds)
    }

    /// Return the raw duration.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Signed stability metric delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationStabilityMetricDelta(i64);

impl RustVerificationStabilityMetricDelta {
    /// Build a signed metric delta.
    #[must_use]
    pub const fn new(delta: i64) -> Self {
        Self(delta)
    }

    /// Return the raw signed delta.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

/// Structured output from a downstream long-running stability fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityRunReceipt {
    /// Command or adapter that produced this receipt.
    pub command: String,
    /// Number of measured iterations.
    pub iterations: RustVerificationStabilityIterationCount,
    /// Total measured duration in seconds.
    pub duration_seconds: RustVerificationStabilityDurationSeconds,
    /// Optional peak resident set size delta in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rss_delta_bytes: Option<RustVerificationStabilityMetricDelta>,
    /// Optional file-descriptor count delta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fd_delta: Option<RustVerificationStabilityMetricDelta>,
    /// Optional thread count delta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_delta: Option<RustVerificationStabilityMetricDelta>,
    /// Optional project-owned state size delta in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_delta_bytes: Option<RustVerificationStabilityMetricDelta>,
    /// Deterministic replay fingerprint, when the run supports replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determinism_hash: Option<String>,
}

impl RustVerificationStabilityRunReceipt {
    /// Build a long-running stability run receipt.
    #[must_use]
    pub fn new(command: impl Into<String>, iterations: u64, duration_seconds: u64) -> Self {
        Self {
            command: command.into(),
            iterations: RustVerificationStabilityIterationCount::new(iterations),
            duration_seconds: RustVerificationStabilityDurationSeconds::new(duration_seconds),
            rss_delta_bytes: None,
            fd_delta: None,
            thread_delta: None,
            state_delta_bytes: None,
            determinism_hash: None,
        }
    }

    /// Attach resource deltas collected by the fixture.
    #[must_use]
    pub const fn with_resource_deltas(
        mut self,
        rss_delta_bytes: i64,
        fd_delta: i64,
        thread_delta: i64,
    ) -> Self {
        self.rss_delta_bytes = Some(RustVerificationStabilityMetricDelta::new(rss_delta_bytes));
        self.fd_delta = Some(RustVerificationStabilityMetricDelta::new(fd_delta));
        self.thread_delta = Some(RustVerificationStabilityMetricDelta::new(thread_delta));
        self
    }

    /// Attach a project-owned state growth delta.
    #[must_use]
    pub const fn with_state_delta_bytes(mut self, state_delta_bytes: i64) -> Self {
        self.state_delta_bytes = Some(RustVerificationStabilityMetricDelta::new(state_delta_bytes));
        self
    }

    /// Attach a deterministic replay fingerprint.
    #[must_use]
    pub fn with_determinism_hash(mut self, determinism_hash: impl Into<String>) -> Self {
        self.determinism_hash = Some(determinism_hash.into());
        self
    }

    /// Convert this run into stable receipt evidence key/value pairs.
    #[must_use]
    pub fn receipt_evidence(&self) -> Vec<(&'static str, String)> {
        let mut evidence = vec![
            ("stability_command", self.command.clone()),
            (
                "iteration_window",
                format!(
                    "{} iterations duration_s={}",
                    self.iterations.as_u64(),
                    self.duration_seconds.as_u64()
                ),
            ),
        ];
        if self.rss_delta_bytes.is_some() || self.fd_delta.is_some() || self.thread_delta.is_some()
        {
            evidence.push((
                "resource_delta",
                format!(
                    "rss_bytes={} fd={} threads={}",
                    self.rss_delta_bytes
                        .map_or(0, RustVerificationStabilityMetricDelta::as_i64),
                    self.fd_delta
                        .map_or(0, RustVerificationStabilityMetricDelta::as_i64),
                    self.thread_delta
                        .map_or(0, RustVerificationStabilityMetricDelta::as_i64)
                ),
            ));
        }
        if let Some(state_delta_bytes) = self.state_delta_bytes {
            evidence.push((
                "state_growth",
                format!("state_delta_bytes={}", state_delta_bytes.as_i64()),
            ));
        }
        if let Some(determinism_hash) = &self.determinism_hash {
            evidence.push(("determinism", format!("hash={determinism_hash}")));
        }
        evidence
    }
}

/// Baseline delta produced by comparing two stability run receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationStabilityBaselineDelta {
    /// Iteration count delta.
    pub iteration_delta: RustVerificationStabilityMetricDelta,
    /// Duration delta in seconds.
    pub duration_delta_seconds: RustVerificationStabilityMetricDelta,
    /// RSS delta drift, when both receipts reported it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rss_delta_bytes: Option<RustVerificationStabilityMetricDelta>,
    /// File descriptor delta drift, when both receipts reported it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fd_delta: Option<RustVerificationStabilityMetricDelta>,
    /// Thread delta drift, when both receipts reported it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_delta: Option<RustVerificationStabilityMetricDelta>,
    /// State size delta drift, when both receipts reported it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_delta_bytes: Option<RustVerificationStabilityMetricDelta>,
    /// Whether deterministic replay hash changed.
    pub determinism_changed: bool,
}

/// Compare the current stability run against a baseline run.
#[must_use]
pub fn compare_rust_verification_stability_runs(
    baseline: &RustVerificationStabilityRunReceipt,
    current: &RustVerificationStabilityRunReceipt,
) -> RustVerificationStabilityBaselineDelta {
    RustVerificationStabilityBaselineDelta {
        iteration_delta: RustVerificationStabilityMetricDelta::new(
            current.iterations.as_u64() as i64 - baseline.iterations.as_u64() as i64,
        ),
        duration_delta_seconds: RustVerificationStabilityMetricDelta::new(
            current.duration_seconds.as_u64() as i64 - baseline.duration_seconds.as_u64() as i64,
        ),
        rss_delta_bytes: optional_delta(baseline.rss_delta_bytes, current.rss_delta_bytes),
        fd_delta: optional_delta(baseline.fd_delta, current.fd_delta),
        thread_delta: optional_delta(baseline.thread_delta, current.thread_delta),
        state_delta_bytes: optional_delta(baseline.state_delta_bytes, current.state_delta_bytes),
        determinism_changed: baseline.determinism_hash.is_some()
            && current.determinism_hash.is_some()
            && baseline.determinism_hash != current.determinism_hash,
    }
}

const fn optional_delta(
    baseline: Option<RustVerificationStabilityMetricDelta>,
    current: Option<RustVerificationStabilityMetricDelta>,
) -> Option<RustVerificationStabilityMetricDelta> {
    match (baseline, current) {
        (Some(baseline), Some(current)) => Some(RustVerificationStabilityMetricDelta::new(
            current.as_i64() - baseline.as_i64(),
        )),
        _ => None,
    }
}
