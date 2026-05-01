//! Verification task planner derived from parser reasoning-tree facts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::model::RustHarnessConfig;
use crate::parser::{
    ParsedRustModule, RustReasoningImportFacts, RustReasoningModuleFacts,
    RustReasoningOwnerBranchFacts, RustReasoningOwnerBranchRole, parse_rust_file,
    rust_reasoning_tree_facts,
};

use super::fingerprint::verification_task_fingerprint;
use super::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationReceiptStatus, RustVerificationTask, RustVerificationTaskKind,
    RustVerificationTaskState, RustVerificationWaiver,
};

struct VerificationTaskSpec {
    kind: RustVerificationTaskKind,
    phase: RustVerificationPhase,
    owner_path: PathBuf,
    owner_namespace: Vec<String>,
    line: Option<usize>,
    reason: &'static str,
    required_receipt: &'static str,
    evidence: Vec<RustVerificationEvidence>,
}

/// Plan parser-native verification tasks for a conventional Rust project.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification(project_root: &Path) -> Result<RustVerificationPlan, String> {
    plan_rust_project_verification_with_config(project_root, &RustHarnessConfig::default())
}

/// Plan parser-native verification tasks with the verification policy embedded
/// in the harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustVerificationPlan, String> {
    plan_rust_project_verification_with_policy(project_root, config, &config.verification_policy)
}

/// Plan parser-native verification tasks with an explicit verification policy.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification_with_policy(
    project_root: &Path,
    config: &RustHarnessConfig,
    policy: &RustVerificationPolicy,
) -> Result<RustVerificationPlan, String> {
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    let package_roots = discover_cargo_package_roots(project_root, &config.ignored_dir_names);
    let package_roots = if should_run_member_scopes(project_root, &package_roots) {
        package_roots
    } else {
        vec![project_root.to_path_buf()]
    };
    let mut tasks = BTreeMap::new();
    for package_root in package_roots {
        let scope = rust_project_harness_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let parsed_modules = parse_scope(&scope, config);
        let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
        collect_profile_tasks(
            &reasoning_tree.package_root,
            &reasoning_tree.modules,
            policy,
            &mut tasks,
        );
        collect_regression_tasks(
            &reasoning_tree.package_root,
            &reasoning_tree.owner_branches,
            policy,
            &mut tasks,
        );
    }
    let mut plan = RustVerificationPlan {
        project_root: project_root.to_path_buf(),
        tasks: tasks.into_values().collect(),
    };
    plan.tasks.sort_by(|left, right| {
        left.package_root
            .cmp(&right.package_root)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    Ok(plan)
}

fn collect_profile_tasks(
    package_root: &Path,
    modules: &[RustReasoningModuleFacts],
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let source_modules = modules
        .iter()
        .filter(|module| module.is_source_module)
        .collect::<Vec<_>>();
    for hint in &policy.profile_hints {
        let owner_path = resolve_hint_owner_path(package_root, hint);
        let Some(module) = source_modules
            .iter()
            .find(|module| module.path == owner_path)
            .copied()
        else {
            push_task(
                tasks,
                policy,
                new_profile_review_task(
                    package_root,
                    owner_path,
                    Vec::new(),
                    "profile hint target is not a parser-known Rust source module",
                    vec![RustVerificationEvidence::new(
                        "hint",
                        format!(
                            "responsibilities={}",
                            responsibility_labels(&hint.responsibilities)
                        ),
                    )],
                    policy,
                ),
            );
            continue;
        };
        collect_profile_conflict_task(package_root, module, hint, policy, tasks);
        collect_skill_tasks_from_profile(package_root, module, hint, policy, tasks);
    }
}

fn collect_profile_conflict_task(
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    if !hint
        .responsibilities
        .contains(&RustOwnerResponsibility::PureDomainLogic)
    {
        return;
    }
    let non_test_owner_deps = non_test_owner_dependency_count(&module.import_summary);
    if module.import_summary.external_imports == 0 && non_test_owner_deps == 0 {
        return;
    }
    let task = new_profile_review_task(
        package_root,
        module.path.clone(),
        module.source_path.namespace_components.clone(),
        "profile declares pure domain logic but parser facts show runtime or owner dependencies",
        vec![
            RustVerificationEvidence::new("profile", responsibility_labels(&hint.responsibilities)),
            RustVerificationEvidence::new(
                "parser",
                format!(
                    "external_imports={} owner_deps={non_test_owner_deps}",
                    module.import_summary.external_imports
                ),
            ),
        ],
        policy,
    );
    push_task(tasks, policy, task);
}

fn collect_skill_tasks_from_profile(
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let responsibilities = &hint.responsibilities;
    if responsibilities.contains(&RustOwnerResponsibility::PublicApi)
        || responsibilities.contains(&RustOwnerResponsibility::LatencySensitive)
    {
        push_task(
            tasks,
            policy,
            new_skill_task(
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Stress,
                    phase: RustVerificationPhase::AfterUnitTestsPass,
                    owner_path: module.path.clone(),
                    owner_namespace: module.source_path.namespace_components.clone(),
                    line: None,
                    reason: "profile declares public or latency-sensitive surface",
                    required_receipt: "stress skill must report p50/p99/p999, load steps, and SLA result for this fingerprint",
                    evidence: vec![RustVerificationEvidence::new(
                        "profile",
                        responsibility_labels(responsibilities),
                    )],
                },
                policy,
            ),
        );
    }
    if responsibilities.contains(&RustOwnerResponsibility::ExternalDependency)
        || responsibilities.contains(&RustOwnerResponsibility::Persistence)
        || responsibilities.contains(&RustOwnerResponsibility::AvailabilityCritical)
    {
        push_task(
            tasks,
            policy,
            new_skill_task(
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Chaos,
                    phase: RustVerificationPhase::BeforeRelease,
                    owner_path: module.path.clone(),
                    owner_namespace: module.source_path.namespace_components.clone(),
                    line: None,
                    reason: "profile declares dependency, persistence, or availability responsibility",
                    required_receipt: "chaos skill must report injected failures, degradation behavior, and recovery result for this fingerprint",
                    evidence: vec![RustVerificationEvidence::new(
                        "profile",
                        responsibility_labels(responsibilities),
                    )],
                },
                policy,
            ),
        );
    }
    if responsibilities.contains(&RustOwnerResponsibility::SecurityBoundary) {
        push_task(
            tasks,
            policy,
            new_skill_task(
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Security,
                    phase: RustVerificationPhase::BeforeRelease,
                    owner_path: module.path.clone(),
                    owner_namespace: module.source_path.namespace_components.clone(),
                    line: None,
                    reason: "profile declares auth, authorization, secret, or trust-boundary logic",
                    required_receipt: "security skill must report scanned attack classes and authorization-boundary result for this fingerprint",
                    evidence: vec![RustVerificationEvidence::new(
                        "profile",
                        responsibility_labels(responsibilities),
                    )],
                },
                policy,
            ),
        );
    }
}

fn collect_regression_tasks(
    package_root: &Path,
    branches: &[RustReasoningOwnerBranchFacts],
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for branch in branches {
        if branch.roles.contains(&RustReasoningOwnerBranchRole::Root) {
            continue;
        }
        let child_count = branch.declared_child_edges.len();
        let dependency_count = non_test_owner_dependency_count(&branch.import_summary);
        if child_count < 3 && dependency_count < 3 {
            continue;
        }
        push_task(
            tasks,
            policy,
            new_skill_task(
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Regression,
                    phase: RustVerificationPhase::ScheduledRegression,
                    owner_path: branch.path.clone(),
                    owner_namespace: branch.owner_namespace.clone(),
                    line: None,
                    reason: "parser facts show a branch coordinating several child modules or local owners",
                    required_receipt: "regression skill must report source growth, dependency drift, and module-cycle status for this fingerprint",
                    evidence: vec![
                        RustVerificationEvidence::new("child_modules", child_count.to_string()),
                        RustVerificationEvidence::new("owner_deps", dependency_count.to_string()),
                    ],
                },
                policy,
            ),
        );
    }
}

fn new_profile_review_task(
    package_root: &Path,
    owner_path: PathBuf,
    owner_namespace: Vec<String>,
    reason: &'static str,
    evidence: Vec<RustVerificationEvidence>,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    new_skill_task(
        package_root,
        VerificationTaskSpec {
            kind: RustVerificationTaskKind::ResponsibilityReview,
            phase: RustVerificationPhase::BeforeVerification,
            owner_path,
            owner_namespace,
            line: None,
            reason,
            required_receipt: "update the verification profile hint to match parser facts, or attach a complete waiver",
            evidence,
        },
        policy,
    )
}

fn new_skill_task(
    package_root: &Path,
    spec: VerificationTaskSpec,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    let fingerprint = verification_task_fingerprint(
        spec.kind,
        package_root,
        &spec.owner_path,
        spec.line,
        &spec.evidence,
    );
    let mut task = RustVerificationTask {
        fingerprint,
        kind: spec.kind,
        state: RustVerificationTaskState::Pending,
        package_root: package_root.to_path_buf(),
        owner_path: spec.owner_path,
        owner_namespace: spec.owner_namespace,
        line: spec.line,
        phase: spec.phase,
        reason: spec.reason.to_string(),
        required_receipt: spec.required_receipt.to_string(),
        evidence: spec.evidence,
        receipt_summary: None,
        waiver_reason: None,
    };
    apply_task_resolution(&mut task, policy);
    task
}

fn apply_task_resolution(task: &mut RustVerificationTask, policy: &RustVerificationPolicy) {
    if let Some(receipt) = matching_receipt(task, policy, RustVerificationReceiptStatus::Passed) {
        task.state = RustVerificationTaskState::Satisfied;
        task.receipt_summary = Some(receipt_summary(receipt));
        return;
    }
    if let Some(waiver) = matching_waiver(task, policy) {
        task.state = RustVerificationTaskState::Waived;
        task.waiver_reason = Some(waiver.reason.clone());
        return;
    }
    if let Some(receipt) = matching_receipt(task, policy, RustVerificationReceiptStatus::Failed) {
        task.state = RustVerificationTaskState::Failed;
        task.receipt_summary = Some(receipt_summary(receipt));
    }
}

fn matching_receipt<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
    status: RustVerificationReceiptStatus,
) -> Option<&'a RustVerificationReceipt> {
    policy.receipts.iter().find(|receipt| {
        receipt.task_fingerprint == task.fingerprint
            && receipt.kind == task.kind
            && receipt.status == status
    })
}

fn matching_waiver<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
) -> Option<&'a RustVerificationWaiver> {
    policy
        .waivers
        .iter()
        .find(|waiver| waiver.task_fingerprint == task.fingerprint && waiver.is_complete())
}

fn receipt_summary(receipt: &RustVerificationReceipt) -> String {
    receipt.evidence_uri.as_ref().map_or_else(
        || receipt.summary.clone(),
        |uri| format!("{} ({uri})", receipt.summary),
    )
}

fn push_task(
    tasks: &mut BTreeMap<String, RustVerificationTask>,
    policy: &RustVerificationPolicy,
    task: RustVerificationTask,
) {
    if policy.disabled_task_kinds.contains(&task.kind) {
        return;
    }
    tasks.entry(task.fingerprint.clone()).or_insert(task);
}

fn parse_scope(
    scope: &crate::RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<ParsedRustModule> {
    discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn resolve_hint_owner_path(package_root: &Path, hint: &RustVerificationProfileHint) -> PathBuf {
    if hint.owner_path.is_absolute() {
        return hint.owner_path.clone();
    }
    package_root.join(&hint.owner_path)
}

fn non_test_owner_dependency_count(imports: &RustReasoningImportFacts) -> usize {
    imports
        .local_owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .count()
}

fn responsibility_labels(
    responsibilities: &std::collections::BTreeSet<RustOwnerResponsibility>,
) -> String {
    responsibilities
        .iter()
        .map(|responsibility| responsibility.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}
