//! Compact verification skill descriptors.

use serde::{Deserialize, Serialize};

/// Compact contract that explains how an Agent skill binding is executed.
///
/// Bindings keep the default verification render quiet. Descriptors are the
/// optional reasoning-tree node an agent can expand when it needs the adapter's
/// execution standard without loading a long Markdown skill.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustVerificationSkillDescriptor {
    /// Stable local or external skill id.
    pub skill_id: String,
    /// Optional adapter name such as `criterion`, `divan`, `iai-callgrind`,
    /// `k6`, or `semgrep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Tool or runtime family used by the adapter.
    pub tool: String,
    /// Compact command template.
    pub command: String,
    /// Short pass/fail standard.
    pub standard: String,
    /// Inputs the Agent must resolve before dispatch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
    /// Criteria that make the run pass.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pass_criteria: Vec<String>,
    /// Receipt fields expected after the run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_fields: Vec<String>,
}

impl RustVerificationSkillDescriptor {
    /// Build a descriptor for a configured skill.
    #[must_use]
    pub fn new(skill_id: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            adapter: None,
            tool: String::new(),
            command: String::new(),
            standard: String::new(),
            required_inputs: Vec::new(),
            pass_criteria: Vec::new(),
            receipt_fields: Vec::new(),
        }
    }

    /// Built-in compact descriptor for the k6 stress adapter.
    ///
    /// The contract intentionally stays short: k6 scenarios define load shape,
    /// thresholds define pass/fail, and the receipt records the latency and SLA
    /// fields the harness already requires.
    #[must_use]
    pub fn k6_stress() -> Self {
        Self::new("rust-verification-stress")
            .with_adapter("k6")
            .with_tool("k6")
            .with_command("k6 run <script>")
            .with_standard("scenarios define load shape; thresholds define pass/fail")
            .with_required_inputs(["script", "target_url", "scenario", "thresholds"])
            .with_pass_criteria(["exit=0", "thresholds=pass"])
            .with_receipt_fields(["p50", "p99", "p999", "load_steps", "sla_result", "artifact"])
    }

    /// Built-in compact descriptor for Criterion-based Rust performance checks.
    ///
    /// Criterion is the statistics-oriented Rust benchmark adapter. Use it for
    /// code-level latency, throughput, and allocation-regression evidence rather
    /// than service-boundary stress tests.
    #[must_use]
    pub fn criterion_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("criterion")
            .with_tool("criterion")
            .with_command("cargo bench")
            .with_standard("statistical benchmark baseline detects runtime regression")
            .with_required_inputs(["bench_target", "baseline", "regression_threshold"])
            .with_pass_criteria(["regression<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
            ])
    }

    /// Built-in compact descriptor for Divan-based Rust performance checks.
    ///
    /// Divan is a modern Rust benchmark adapter over `cargo bench`; keep it in
    /// the Rust-native performance lane rather than the service stress lane.
    #[must_use]
    pub fn divan_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("divan")
            .with_tool("divan")
            .with_command("cargo bench")
            .with_standard("sampled Rust benchmark summary stays within regression threshold")
            .with_required_inputs(["bench_target", "baseline", "regression_threshold"])
            .with_pass_criteria(["median_or_mean_delta<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
                "samples",
                "iters",
            ])
    }

    /// Built-in compact descriptor for iai-callgrind Rust performance checks.
    ///
    /// iai-callgrind is the deterministic CI-oriented adapter for instruction,
    /// cache, and allocation profiles. It complements wall-clock benchmarks when
    /// the Agent needs lower-noise regression evidence.
    #[must_use]
    pub fn iai_callgrind_performance() -> Self {
        Self::new("rust-verification-performance")
            .with_adapter("iai-callgrind")
            .with_tool("iai-callgrind")
            .with_command("cargo bench")
            .with_standard("instruction/cache/allocation metrics stay within regression threshold")
            .with_required_inputs(["bench_target", "baseline", "metric", "regression_threshold"])
            .with_pass_criteria(["metric_delta<=threshold"])
            .with_receipt_fields([
                "benchmark_command",
                "baseline",
                "regression_threshold",
                "latency_or_throughput",
                "allocation_profile",
                "profile_artifact",
                "instructions",
                "cache_misses",
            ])
    }

    /// Attach an adapter label for this descriptor.
    #[must_use]
    pub fn with_adapter(mut self, adapter: impl Into<String>) -> Self {
        self.adapter = Some(adapter.into());
        self
    }

    /// Set the tool family.
    #[must_use]
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = tool.into();
        self
    }

    /// Set the command template.
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    /// Set the compact execution standard.
    #[must_use]
    pub fn with_standard(mut self, standard: impl Into<String>) -> Self {
        self.standard = standard.into();
        self
    }

    /// Set required adapter inputs.
    #[must_use]
    pub fn with_required_inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_inputs = inputs.into_iter().map(Into::into).collect();
        self
    }

    /// Set pass criteria.
    #[must_use]
    pub fn with_pass_criteria<I, S>(mut self, criteria: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.pass_criteria = criteria.into_iter().map(Into::into).collect();
        self
    }

    /// Set receipt fields.
    #[must_use]
    pub fn with_receipt_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.receipt_fields = fields.into_iter().map(Into::into).collect();
        self
    }

    pub(crate) fn compact_label(&self) -> String {
        self.adapter
            .as_deref()
            .map(str::trim)
            .filter(|adapter| !adapter.is_empty())
            .map_or_else(
                || self.skill_id.clone(),
                |adapter| format!("{}@{adapter}", self.skill_id),
            )
    }

    pub(crate) fn fingerprint_material(&self) -> String {
        format!(
            "tool={};command={};standard={};inputs={};pass={};receipt={}",
            self.tool,
            self.command,
            self.standard,
            self.required_inputs.join(","),
            self.pass_criteria.join(","),
            self.receipt_fields.join(",")
        )
    }
}
