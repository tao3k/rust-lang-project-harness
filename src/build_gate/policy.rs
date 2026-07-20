//! Downstream and workspace build-gate policy configuration.

use crate::model::RustHarnessConfig;

use super::RustProjectHarnessDependencyBaseline;

/// Downstream crate-owned policy consumed by a thin `build.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessDownstreamPolicy {
    gate_label: String,
    config: RustHarnessConfig,
    dependency_baseline: Option<RustProjectHarnessDependencyBaseline>,
}

impl RustProjectHarnessDownstreamPolicy {
    /// Create a downstream policy around a complete harness config.
    /// Return the build-gate label.
    #[must_use]
    pub fn new(gate_label: impl Into<String>, config: RustHarnessConfig) -> Self {
        Self {
            gate_label: gate_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Return the harness config.
    #[must_use]
    pub fn gate_label(&self) -> &str {
        &self.gate_label
    }

    /// Attach an exact dependency baseline.
    #[must_use]
    pub fn config(&self) -> &RustHarnessConfig {
        &self.config
    }

    /// Return the optional dependency baseline.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: RustProjectHarnessDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&RustProjectHarnessDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }
}

/// Workspace-owned policy baseline shared by multiple downstream crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessWorkspacePolicy {
    workspace_label: String,
    config: RustHarnessConfig,
    dependency_baseline: Option<RustProjectHarnessDependencyBaseline>,
}

impl RustProjectHarnessWorkspacePolicy {
    /// Create a workspace policy around a shared harness config.
    /// Return the workspace label.
    #[must_use]
    pub fn new(workspace_label: impl Into<String>, config: RustHarnessConfig) -> Self {
        Self {
            workspace_label: workspace_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Return the shared harness config.
    #[must_use]
    pub fn workspace_label(&self) -> &str {
        &self.workspace_label
    }

    /// Attach the dependency baseline inherited by member policies.
    #[must_use]
    pub fn config(&self) -> &RustHarnessConfig {
        &self.config
    }

    /// Return the optional shared dependency baseline.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: RustProjectHarnessDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    /// Derive a member policy from the shared config.
    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&RustProjectHarnessDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }

    /// Derive a member policy after applying a config transformation.
    #[must_use]
    pub fn member_crate(
        &self,
        crate_label: impl Into<String>,
    ) -> RustProjectHarnessDownstreamPolicy {
        self.attach_dependency_baseline(RustProjectHarnessDownstreamPolicy::new(
            self.member_gate_label(crate_label),
            self.config.clone(),
        ))
    }

    #[must_use]
    pub fn member_crate_with_config<F>(
        &self,
        crate_label: impl Into<String>,
        configure: F,
    ) -> RustProjectHarnessDownstreamPolicy
    where
        F: FnOnce(RustHarnessConfig) -> RustHarnessConfig,
    {
        self.attach_dependency_baseline(RustProjectHarnessDownstreamPolicy::new(
            self.member_gate_label(crate_label),
            configure(self.config.clone()),
        ))
    }

    fn member_gate_label(&self, crate_label: impl Into<String>) -> String {
        format!("{}::{}", self.workspace_label, crate_label.into())
    }

    fn attach_dependency_baseline(
        &self,
        policy: RustProjectHarnessDownstreamPolicy,
    ) -> RustProjectHarnessDownstreamPolicy {
        match self.dependency_baseline.clone() {
            Some(dependency_baseline) => policy.with_dependency_baseline(dependency_baseline),
            None => policy,
        }
    }
}
