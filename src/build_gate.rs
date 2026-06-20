//! Build-script entrypoints for `cargo check` project harness gates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::model::{RustHarnessConfig, RustHarnessReport};
use crate::runner::run_rust_project_harness_with_config;
use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};

/// Stable schema id for downstream policy receipt projections.
pub const RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID: &str =
    "rust-lang-project-harness.downstream-policy-receipt";

/// Current downstream policy receipt schema version.
pub const RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION: &str = "1";

/// Downstream crate-owned policy consumed by a thin `build.rs`.
///
/// Downstream projects should construct this value from crate-local policy
/// modules, then pass it to
/// [`assert_rust_project_harness_downstream_policy_from_env`]. This keeps
/// `build.rs` small while still letting larger projects split policy into
/// owners, verification, receipts, reports, and rule modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessDownstreamPolicy {
    gate_label: String,
    config: RustHarnessConfig,
    dependency_baseline: Option<RustProjectHarnessDependencyBaseline>,
}

impl RustProjectHarnessDownstreamPolicy {
    /// Create a downstream policy wrapper around a complete harness config.
    #[must_use]
    pub fn new(gate_label: impl Into<String>, config: RustHarnessConfig) -> Self {
        Self {
            gate_label: gate_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Human-readable crate label used in build-gate panic messages.
    #[must_use]
    pub fn gate_label(&self) -> &str {
        &self.gate_label
    }

    /// Complete project harness config assembled by downstream policy modules.
    #[must_use]
    pub fn config(&self) -> &RustHarnessConfig {
        &self.config
    }

    /// Attach a Cargo.lock dependency baseline to this downstream gate.
    ///
    /// This lets downstream workspaces make git rev/version drift part of the
    /// same `build.rs` semantic contract that already checks owners and
    /// verification evidence.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: RustProjectHarnessDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    /// Optional dependency baseline asserted by this downstream gate.
    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&RustProjectHarnessDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }
}

/// Cargo.lock dependency baseline shared by downstream build gates.
///
/// Use this when a downstream workspace must guarantee that a package resolves
/// to one exact version and one git source/rev across all member crates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustProjectHarnessDependencyBaseline {
    packages: Vec<RustProjectHarnessDependencyBaselinePackage>,
}

impl RustProjectHarnessDependencyBaseline {
    /// Create an empty dependency baseline.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Require one git package to resolve to an exact version and source
    /// fragment, such as `rev=<commit>`.
    #[must_use]
    pub fn require_git_package(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        source_contains: impl Into<String>,
    ) -> Self {
        self.packages
            .push(RustProjectHarnessDependencyBaselinePackage {
                name: name.into(),
                version: version.into(),
                source_contains: source_contains.into(),
            });
        self
    }

    /// Required packages in insertion order.
    #[must_use]
    pub fn packages(&self) -> &[RustProjectHarnessDependencyBaselinePackage] {
        &self.packages
    }
}

/// One exact Cargo.lock package requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessDependencyBaselinePackage {
    name: String,
    version: String,
    source_contains: String,
}

impl RustProjectHarnessDependencyBaselinePackage {
    /// Package name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact package version expected in Cargo.lock.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Required source fragment expected in Cargo.lock.
    #[must_use]
    pub fn source_contains(&self) -> &str {
        &self.source_contains
    }
}

/// Agent-facing receipt for a downstream build-gate policy.
///
/// This is a non-panicking observation surface for CI, agents, and workspace
/// policy modules that need to inspect the same semantic contract enforced by
/// [`assert_rust_project_harness_downstream_policy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessDownstreamPolicyReceipt {
    /// Stable receipt schema id.
    pub schema_id: String,
    /// Stable receipt schema version.
    pub schema_version: String,
    /// Human-readable gate label configured by the downstream policy.
    pub gate_label: String,
    /// Dependency baseline requirements inherited by this member policy.
    pub dependency_baseline_packages: Vec<RustProjectHarnessDependencyBaselinePackageReceipt>,
    /// Number of active verification tasks in the generated plan.
    pub active_verification_task_count: usize,
    /// Number of active performance verification tasks.
    pub performance_task_count: usize,
    /// Number of active stability verification tasks.
    pub stability_task_count: usize,
    /// Whether the generated plan requires the performance report artifact.
    pub performance_report_obligation: bool,
    /// Whether the generated plan requires the stability report artifact.
    pub stability_report_obligation: bool,
    /// Report obligations emitted by the generated verification plan.
    pub report_obligations: Vec<RustProjectHarnessReportObligationReceipt>,
}

/// Receipt projection of one dependency baseline package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessDependencyBaselinePackageReceipt {
    /// Package name.
    pub name: String,
    /// Exact package version expected in Cargo.lock.
    pub version: String,
    /// Required source fragment expected in Cargo.lock.
    pub source_contains: String,
}

/// Receipt projection of one verification report obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessReportObligationReceipt {
    /// Stable report contract key.
    pub key: String,
    /// Harness renderer or index builder that produces the report payload.
    pub renderer: String,
    /// Recommended artifact filename for embedding projects.
    pub suggested_artifact_name: String,
    /// Why this report should be persisted for later comparison.
    pub reason: String,
    /// Active task family keys covered by this report.
    pub task_kinds: Vec<String>,
    /// Active task fingerprints covered by this report.
    pub task_fingerprints: Vec<String>,
}

/// Workspace-owned policy baseline shared by multiple downstream crates.
///
/// Downstream workspaces should keep common rules, receipts, and verification
/// defaults in this value, then derive crate policies through
/// [`RustProjectHarnessWorkspacePolicy::member_crate`] or
/// [`RustProjectHarnessWorkspacePolicy::member_crate_with_config`]. This keeps
/// member `build.rs` files thin without forcing every crate to duplicate the
/// workspace policy tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessWorkspacePolicy {
    workspace_label: String,
    config: RustHarnessConfig,
    dependency_baseline: Option<RustProjectHarnessDependencyBaseline>,
}

impl RustProjectHarnessWorkspacePolicy {
    /// Create a workspace policy wrapper around a shared harness config.
    #[must_use]
    pub fn new(workspace_label: impl Into<String>, config: RustHarnessConfig) -> Self {
        Self {
            workspace_label: workspace_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Human-readable workspace label used as the prefix for member gates.
    #[must_use]
    pub fn workspace_label(&self) -> &str {
        &self.workspace_label
    }

    /// Shared harness config used as the baseline for member crate policies.
    #[must_use]
    pub fn config(&self) -> &RustHarnessConfig {
        &self.config
    }

    /// Attach a dependency baseline shared by all derived member crates.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: RustProjectHarnessDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    /// Optional dependency baseline shared by derived member crate policies.
    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&RustProjectHarnessDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }

    /// Derive a member crate policy from the shared workspace config.
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

    /// Derive a member crate policy and apply crate-local overrides.
    ///
    /// The shared workspace config is cloned before the override closure runs,
    /// so member-specific owners or waivers cannot mutate the common baseline.
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

/// Assert a complete downstream policy from `CARGO_MANIFEST_DIR`.
///
/// This is the preferred entrypoint for downstream crates whose policy is too
/// large to live directly in `build.rs`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the cargo-check policy
/// gate fails, or when semantic verification coverage is incomplete.
#[track_caller]
pub fn assert_rust_project_harness_downstream_policy_from_env(
    policy: &RustProjectHarnessDownstreamPolicy,
) -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_downstream_policy(&root, policy)
}

/// Assert a complete downstream policy from an explicit project root.
///
/// # Panics
///
/// Panics when the cargo-check policy gate fails, or when semantic
/// verification coverage is incomplete.
#[track_caller]
pub fn assert_rust_project_harness_downstream_policy(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
) -> RustHarnessReport {
    let report = assert_downstream_cargo_check_clean_with_guidance(project_root, policy);
    assert_rust_project_harness_verification_with_config(
        project_root,
        policy.config(),
        policy.gate_label(),
    );
    if let Some(dependency_baseline) = policy.dependency_baseline() {
        assert_rust_project_harness_dependency_baseline(
            project_root,
            dependency_baseline,
            policy.gate_label(),
        );
    }
    report
}

/// Build an agent-facing receipt for a downstream policy without asserting it.
///
/// The receipt uses the same verification planner as the build gate, but does
/// not run cargo-check assertions or dependency-baseline lockfile checks. Use
/// this when a downstream CI job or agent wants a typed observation surface
/// before the panic-oriented `build.rs` gate fires.
pub fn rust_project_harness_downstream_policy_receipt(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
) -> Result<RustProjectHarnessDownstreamPolicyReceipt, String> {
    let plan = plan_rust_project_verification_with_config(project_root, policy.config())?;
    Ok(downstream_policy_receipt_from_plan(policy, &plan))
}

pub(crate) fn downstream_policy_receipt_from_plan(
    policy: &RustProjectHarnessDownstreamPolicy,
    plan: &RustVerificationPlan,
) -> RustProjectHarnessDownstreamPolicyReceipt {
    RustProjectHarnessDownstreamPolicyReceipt {
        schema_id: RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID.to_string(),
        schema_version: RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION.to_string(),
        gate_label: policy.gate_label().to_string(),
        dependency_baseline_packages: policy
            .dependency_baseline()
            .into_iter()
            .flat_map(RustProjectHarnessDependencyBaseline::packages)
            .map(
                |package| RustProjectHarnessDependencyBaselinePackageReceipt {
                    name: package.name().to_string(),
                    version: package.version().to_string(),
                    source_contains: package.source_contains().to_string(),
                },
            )
            .collect(),
        active_verification_task_count: plan.active_tasks().len(),
        performance_task_count: active_task_count(plan, RustVerificationTaskKind::Performance),
        stability_task_count: active_task_count(plan, RustVerificationTaskKind::Stability),
        performance_report_obligation: has_report_obligation(plan, "performance_index_json"),
        stability_report_obligation: has_report_obligation(plan, "stability_index_json"),
        report_obligations: plan
            .report_obligations
            .iter()
            .map(|obligation| RustProjectHarnessReportObligationReceipt {
                key: obligation.key.clone(),
                renderer: obligation.renderer.clone(),
                suggested_artifact_name: obligation.suggested_artifact_name.clone(),
                reason: obligation.reason.clone(),
                task_kinds: obligation
                    .task_kinds
                    .iter()
                    .map(|kind| verification_task_kind_key(*kind).to_string())
                    .collect(),
                task_fingerprints: obligation.task_fingerprints.clone(),
            })
            .collect(),
    }
}

/// Render a downstream policy receipt as structured JSON for evidence files.
///
/// # Errors
///
/// Returns a serialization error if the receipt cannot be encoded as JSON.
pub fn render_rust_project_harness_downstream_policy_receipt_json(
    receipt: &RustProjectHarnessDownstreamPolicyReceipt,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(receipt)
}

/// Assert an exact Cargo.lock dependency baseline from a downstream gate.
///
/// The lockfile is searched from `project_root` upward, so member crates in a
/// Cargo workspace can share the workspace root `Cargo.lock`.
///
/// # Panics
///
/// Panics when no `Cargo.lock` is found, the lockfile cannot be parsed, or any
/// required package resolves to a missing, duplicate, wrong-version, or
/// wrong-source entry.
#[track_caller]
pub fn assert_rust_project_harness_dependency_baseline(
    project_root: &Path,
    dependency_baseline: &RustProjectHarnessDependencyBaseline,
    gate_label: &str,
) {
    if dependency_baseline.packages().is_empty() {
        return;
    }
    let lockfile_path = find_cargo_lock(project_root).unwrap_or_else(|| {
        panic!(
            "{gate_label} dependency baseline: Cargo.lock not found from {}\n{}",
            project_root.display(),
            dependency_baseline_agent_guidance(gate_label)
        )
    });
    println!("cargo:rerun-if-changed={}", lockfile_path.display());
    let lockfile = cargo_lock::Lockfile::load(&lockfile_path).unwrap_or_else(|error| {
        panic!(
            "{gate_label} dependency baseline: failed to parse {}: {error}\n{}",
            lockfile_path.display(),
            dependency_baseline_agent_guidance(gate_label)
        )
    });

    for required_package in dependency_baseline.packages() {
        assert_dependency_baseline_package(&lockfile, required_package, gate_label, &lockfile_path);
    }
}

/// Assert a project harness run from a Cargo build script.
///
/// This runs during Cargo build-script execution, so `cargo check` surfaces the
/// parser-native harness policy before Cargo reaches the test/evaluation layer.
/// By default, non-blocking agent advice is also treated as repair feedback.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_build_clean(project_root: &Path) -> RustHarnessReport {
    let config = RustHarnessConfig::default();
    emit_cargo_rerun_inputs(project_root, &config);
    let report = run_rust_project_harness_with_config(project_root, &config)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_build_report_clean(&report, &config);
    report
}

/// Assert a configured project harness run from a Cargo build script.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    emit_cargo_rerun_inputs(project_root, config);
    let report = run_rust_project_harness_with_config(project_root, config)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_build_report_clean(&report, config);
    report
}

/// Assert a project harness run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_from_env() -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_build_clean(&root)
}

/// Assert a configured project harness run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_from_env_with_config(
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_build_clean_with_config(&root, config)
}

/// Assert a project harness policy run from a Cargo build script during `cargo check`.
///
/// This is the preferred downstream entrypoint. The older
/// `assert_rust_project_harness_build_clean(...)` name remains as a
/// compatibility alias.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean(project_root: &Path) -> RustHarnessReport {
    assert_rust_project_harness_build_clean(project_root)
}

/// Assert a configured cargo-check project harness policy run.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    assert_rust_project_harness_build_clean_with_config(project_root, config)
}

/// Assert a cargo-check project harness policy run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_from_env() -> RustHarnessReport {
    assert_rust_project_harness_build_clean_from_env()
}

/// Assert a configured cargo-check project harness policy run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_from_env_with_config(
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    assert_rust_project_harness_build_clean_from_env_with_config(config)
}

/// Assert that a cargo-check build gate has active semantic verification tasks.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, the verification plan cannot be
/// built, or the configured plan lacks active verification tasks/reports.
#[track_caller]
pub fn assert_rust_project_harness_verification_from_env_with_config(
    config: &RustHarnessConfig,
    gate_label: &str,
) {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_verification_with_config(&root, config, gate_label);
}

/// Assert that a cargo-check build gate has active semantic verification tasks.
///
/// This mirrors a Clippy-style build-script gate: downstream crates pass their
/// harness config through `build.rs`, and Cargo surfaces missing semantic
/// verification coverage during `cargo check`/`cargo test` compilation.
///
/// # Panics
///
/// Panics when the verification plan cannot be built, or the configured plan
/// lacks active verification tasks/reports.
#[track_caller]
pub fn assert_rust_project_harness_verification_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
    gate_label: &str,
) {
    let plan =
        plan_rust_project_verification_with_config(project_root, config).unwrap_or_else(|error| {
            panic!(
                "{gate_label} verification plan: {error}\n{}",
                downstream_build_gate_agent_guidance(gate_label)
            )
        });
    assert_active_verification_task(&plan, gate_label, RustVerificationTaskKind::Performance);
    assert_active_verification_task(&plan, gate_label, RustVerificationTaskKind::Stability);
    assert_verification_report_obligation(&plan, gate_label, "performance_index_json");
    assert_verification_report_obligation(&plan, gate_label, "stability_index_json");
}

fn assert_downstream_cargo_check_clean_with_guidance(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
) -> RustHarnessReport {
    emit_cargo_rerun_inputs(project_root, policy.config());
    let report = run_rust_project_harness_with_config(project_root, policy.config())
        .unwrap_or_else(|error| {
            panic!(
                "{} cargo-check build gate: {error}\n{}",
                policy.gate_label(),
                downstream_build_gate_agent_guidance(policy.gate_label())
            )
        });
    assert_build_report_clean_with_agent_guidance(&report, policy.config(), policy.gate_label());
    report
}

fn assert_build_report_clean_with_agent_guidance(
    report: &RustHarnessReport,
    config: &RustHarnessConfig,
    gate_label: &str,
) {
    if !report.is_clean() {
        panic!(
            "{}\n{}",
            crate::render_rust_project_harness(report),
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
    if !config_allows_agent_advice(config) {
        let rendered = crate::render_rust_project_harness_advice(report);
        if !rendered.is_empty() {
            panic!(
                "{rendered}\n{}",
                downstream_build_gate_agent_guidance(gate_label)
            );
        }
    }
}

fn assert_active_verification_task(
    plan: &crate::verification::RustVerificationPlan,
    gate_label: &str,
    kind: RustVerificationTaskKind,
) {
    if !plan
        .tasks
        .iter()
        .any(|task| task.kind == kind && task.is_active())
    {
        panic!(
            "{gate_label} build gate must configure active {kind:?} verification tasks\n{}",
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
}

fn active_task_count(
    plan: &crate::verification::RustVerificationPlan,
    kind: RustVerificationTaskKind,
) -> usize {
    plan.tasks
        .iter()
        .filter(|task| task.kind == kind && task.is_active())
        .count()
}

fn assert_verification_report_obligation(
    plan: &crate::verification::RustVerificationPlan,
    gate_label: &str,
    key: &str,
) {
    if !has_report_obligation(plan, key) {
        panic!(
            "{gate_label} build gate must require a {key} report\n{}",
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
}

fn has_report_obligation(plan: &crate::verification::RustVerificationPlan, key: &str) -> bool {
    plan.report_obligations
        .iter()
        .any(|obligation| obligation.key == key)
}

pub(crate) fn verification_task_kind_key(kind: RustVerificationTaskKind) -> &'static str {
    match kind {
        RustVerificationTaskKind::Stress => "stress",
        RustVerificationTaskKind::Performance => "performance",
        RustVerificationTaskKind::Stability => "stability",
        RustVerificationTaskKind::Chaos => "chaos",
        RustVerificationTaskKind::Security => "security",
        RustVerificationTaskKind::Regression => "regression",
        RustVerificationTaskKind::ResponsibilityReview => "responsibility_review",
    }
}

fn assert_dependency_baseline_package(
    lockfile: &cargo_lock::Lockfile,
    required_package: &RustProjectHarnessDependencyBaselinePackage,
    gate_label: &str,
    lockfile_path: &Path,
) {
    let package_matches = lockfile
        .packages
        .iter()
        .filter(|package| package.name.to_string() == required_package.name())
        .collect::<Vec<_>>();
    if package_matches.len() != 1 {
        panic!(
            "{gate_label} dependency baseline: {} requires exactly one Cargo.lock entry in {}; found {}\nexpected: {}\nactual:\n{}\n{}",
            required_package.name(),
            lockfile_path.display(),
            package_matches.len(),
            render_required_dependency_baseline_package(required_package),
            render_dependency_baseline_package_matches(&package_matches),
            dependency_baseline_agent_guidance(gate_label)
        );
    }

    let package = package_matches[0];
    let actual_version = package.version.to_string();
    let actual_source = package
        .source
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "<none>".to_string());
    if actual_version != required_package.version()
        || !actual_source.contains(required_package.source_contains())
    {
        panic!(
            "{gate_label} dependency baseline: {} resolved to an unexpected Cargo.lock entry in {}\nexpected: {}\nactual: {}\n{}",
            required_package.name(),
            lockfile_path.display(),
            render_required_dependency_baseline_package(required_package),
            render_dependency_baseline_package(package),
            dependency_baseline_agent_guidance(gate_label)
        );
    }
}

fn find_cargo_lock(project_root: &Path) -> Option<PathBuf> {
    let mut current = Some(project_root);
    while let Some(root) = current {
        let candidate = root.join("Cargo.lock");
        if candidate.exists() {
            return Some(candidate);
        }
        current = root.parent();
    }
    None
}

fn render_required_dependency_baseline_package(
    package: &RustProjectHarnessDependencyBaselinePackage,
) -> String {
    format!(
        "{} {} source contains {}",
        package.name(),
        package.version(),
        package.source_contains()
    )
}

fn render_dependency_baseline_package_matches(packages: &[&cargo_lock::Package]) -> String {
    if packages.is_empty() {
        return "- <none>".to_string();
    }
    packages
        .iter()
        .map(|package| format!("- {}", render_dependency_baseline_package(package)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_dependency_baseline_package(package: &cargo_lock::Package) -> String {
    let source = package
        .source
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "<none>".to_string());
    format!("{} {} source {source}", package.name, package.version)
}

fn downstream_build_gate_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[rust-harness-agent-guidance]
gate: {gate_label}
trigger: cargo test runs the member build.rs before tests; keep rust-lang-project-harness under [build-dependencies].
repair:
- keep build.rs thin and call assert_rust_project_harness_downstream_policy_from_env.
- in a workspace, put common policy in the root harness/ module tree.
- construct RustProjectHarnessWorkspacePolicy once, then derive members with member_crate or member_crate_with_config.
- add crate-local owners, receipts, waivers, or report obligations in the member override only.
- rerun cargo test after updating policy or evidence.
"
    )
}

fn dependency_baseline_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[rust-harness-dependency-guidance]
gate: {gate_label}
trigger: Cargo.lock dependency baseline drift.
repair:
- update the workspace dependency declaration that still pins the old version or git rev.
- if a transitive crate pins the old rev, upgrade that crate first instead of overriding the lockfile by hand.
- keep the baseline in shared workspace policy and derive member gates from RustProjectHarnessWorkspacePolicy.
- rerun cargo update for the affected package, then cargo tree -i <package> --workspace.
- rerun cargo test so build.rs verifies the repaired lockfile.
"
    )
}

fn cargo_manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is not set"))
}

fn assert_build_report_clean(report: &RustHarnessReport, config: &RustHarnessConfig) {
    report.assert_clean();
    if !config_allows_agent_advice(config) {
        report.assert_no_advisory_findings();
    }
}

fn config_allows_agent_advice(config: &RustHarnessConfig) -> bool {
    has_explanation(config.cargo_check_advice_allow_explanation.as_deref())
        || has_explanation(config.agent_advice_allow_explanation.as_deref())
}

fn has_explanation(explanation: Option<&str>) -> bool {
    explanation.is_some_and(|explanation| !explanation.trim().is_empty())
}

fn emit_cargo_rerun_inputs(project_root: &Path, config: &RustHarnessConfig) {
    for path in rerun_inputs(project_root, config) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn rerun_inputs(project_root: &Path, config: &RustHarnessConfig) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::from([
        project_root.join("Cargo.toml"),
        project_root.join("Cargo.lock"),
        project_root.join("build.rs"),
    ]);
    let scope = rust_project_harness_scope(
        project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    for path in scope.monitored_paths() {
        paths.insert(path);
    }
    for path in discover_rust_files(
        &[project_root.to_path_buf()],
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    ) {
        paths.insert(path);
    }
    paths.retain(|path| path.exists());
    paths
}
