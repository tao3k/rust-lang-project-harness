//! Verification task planner derived from parser reasoning-tree facts.

use std::collections::{BTreeMap, BTreeSet};
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
    RustVerificationReceiptStatus, RustVerificationRequirement, RustVerificationResolutionNote,
    RustVerificationTask, RustVerificationTaskContract, RustVerificationTaskKind,
    RustVerificationTaskState, RustVerificationWaiver,
};

struct VerificationTaskSpec {
    kind: RustVerificationTaskKind,
    owner_path: PathBuf,
    owner_namespace: Vec<String>,
    line: Option<usize>,
    reason: String,
    contract: RustVerificationTaskContract,
    evidence: Vec<RustVerificationEvidence>,
}

struct ProfileReviewTaskSpec<'a> {
    owner_path: PathBuf,
    owner_namespace: Vec<String>,
    reason: &'static str,
    evidence: Vec<RustVerificationEvidence>,
    hint: Option<&'a RustVerificationProfileHint>,
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
    let mut matched_profile_hints = BTreeSet::new();
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
            project_root,
            &reasoning_tree.package_root,
            &reasoning_tree.modules,
            policy,
            &mut matched_profile_hints,
            &mut tasks,
        );
        collect_regression_tasks(
            project_root,
            &reasoning_tree.package_root,
            &reasoning_tree.owner_branches,
            policy,
            &mut tasks,
        );
    }
    collect_unmatched_profile_hints(project_root, policy, &matched_profile_hints, &mut tasks);
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
    project_root: &Path,
    package_root: &Path,
    modules: &[RustReasoningModuleFacts],
    policy: &RustVerificationPolicy,
    matched_profile_hints: &mut BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let source_modules = modules
        .iter()
        .filter(|module| module.is_source_module)
        .collect::<Vec<_>>();
    for (hint_index, hint) in policy.profile_hints.iter().enumerate() {
        let Some(module) = matching_hint_module(project_root, package_root, &source_modules, hint)
        else {
            continue;
        };
        matched_profile_hints.insert(hint_index);
        collect_profile_conflict_task(project_root, package_root, module, hint, policy, tasks);
        collect_skill_tasks_from_profile(project_root, package_root, module, hint, policy, tasks);
    }
}

fn collect_unmatched_profile_hints(
    project_root: &Path,
    policy: &RustVerificationPolicy,
    matched_profile_hints: &BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for (hint_index, hint) in policy.profile_hints.iter().enumerate() {
        if matched_profile_hints.contains(&hint_index) {
            continue;
        }
        let owner_path = if hint.owner_path.is_absolute() {
            hint.owner_path.clone()
        } else {
            project_root.join(&hint.owner_path)
        };
        push_task(
            tasks,
            policy,
            new_profile_review_task(
                project_root,
                project_root,
                ProfileReviewTaskSpec {
                    owner_path,
                    owner_namespace: Vec::new(),
                    reason: "profile hint target is not a parser-known Rust source module",
                    evidence: vec![RustVerificationEvidence::new(
                        "hint",
                        format!(
                            "responsibilities={}",
                            responsibility_labels(&hint.responsibilities)
                        ),
                    )],
                    hint: Some(hint),
                },
                policy,
            ),
        );
    }
}

fn collect_profile_conflict_task(
    project_root: &Path,
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
        project_root,
        package_root,
        ProfileReviewTaskSpec {
            owner_path: module.path.clone(),
            owner_namespace: module.source_path.namespace_components.clone(),
            reason: "profile declares pure domain logic but parser facts show runtime or owner dependencies",
            evidence: vec![
                RustVerificationEvidence::new(
                    "profile",
                    responsibility_labels(&hint.responsibilities),
                ),
                RustVerificationEvidence::new(
                    "parser",
                    format!(
                        "external_imports={} owner_deps={non_test_owner_deps}",
                        module.import_summary.external_imports
                    ),
                ),
            ],
            hint: Some(hint),
        },
        policy,
    );
    push_task(tasks, policy, task);
}

fn collect_skill_tasks_from_profile(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let responsibilities = &hint.responsibilities;
    let uses_owner_task_override = hint.task_kinds.is_some();
    for kind in task_kinds_for_profile(hint, policy) {
        push_task(
            tasks,
            policy,
            new_skill_task(
                project_root,
                package_root,
                VerificationTaskSpec {
                    kind,
                    owner_path: module.path.clone(),
                    owner_namespace: module.source_path.namespace_components.clone(),
                    line: None,
                    reason: profile_task_reason(kind, responsibilities, uses_owner_task_override),
                    contract: task_contract_for_profile(policy, Some(hint), kind),
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
    project_root: &Path,
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
                project_root,
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Regression,
                    owner_path: branch.path.clone(),
                    owner_namespace: branch.owner_namespace.clone(),
                    line: None,
                    reason: "parser facts show a branch coordinating several child modules or local owners"
                        .to_string(),
                    contract: task_contract_for_profile(
                        policy,
                        None,
                        RustVerificationTaskKind::Regression,
                    ),
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
    project_root: &Path,
    package_root: &Path,
    spec: ProfileReviewTaskSpec<'_>,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    new_skill_task(
        project_root,
        package_root,
        VerificationTaskSpec {
            kind: RustVerificationTaskKind::ResponsibilityReview,
            owner_path: spec.owner_path,
            owner_namespace: spec.owner_namespace,
            line: None,
            reason: spec.reason.to_string(),
            contract: task_contract_for_profile(
                policy,
                spec.hint,
                RustVerificationTaskKind::ResponsibilityReview,
            ),
            evidence: spec.evidence,
        },
        policy,
    )
}

fn new_skill_task(
    project_root: &Path,
    package_root: &Path,
    spec: VerificationTaskSpec,
    policy: &RustVerificationPolicy,
) -> RustVerificationTask {
    let fingerprint = verification_task_fingerprint(
        spec.kind,
        project_root,
        package_root,
        &spec.owner_path,
        spec.line,
        &spec.contract.required_evidence,
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
        phase: spec.contract.phase,
        reason: spec.reason,
        required_receipt: spec.contract.required_receipt,
        required_evidence: spec.contract.required_evidence,
        evidence: spec.evidence,
        resolution_notes: Vec::new(),
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
    if let Some(waiver) = incomplete_matching_waiver(task, policy) {
        task.resolution_notes
            .push(RustVerificationResolutionNote::new(
                "waiver",
                incomplete_waiver_detail(waiver),
            ));
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

fn incomplete_matching_waiver<'a>(
    task: &RustVerificationTask,
    policy: &'a RustVerificationPolicy,
) -> Option<&'a RustVerificationWaiver> {
    policy
        .waivers
        .iter()
        .find(|waiver| waiver.task_fingerprint == task.fingerprint && !waiver.is_complete())
}

fn incomplete_waiver_detail(waiver: &RustVerificationWaiver) -> String {
    let mut missing_fields = Vec::new();
    if waiver.owner.trim().is_empty() {
        missing_fields.push("owner");
    }
    if waiver.reason.trim().is_empty() {
        missing_fields.push("reason");
    }
    if waiver.expires_at.trim().is_empty() {
        missing_fields.push("expires_at");
    }
    format!("incomplete: missing {}", missing_fields.join(", "))
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

fn matching_hint_module<'a>(
    project_root: &Path,
    package_root: &Path,
    modules: &[&'a RustReasoningModuleFacts],
    hint: &RustVerificationProfileHint,
) -> Option<&'a RustReasoningModuleFacts> {
    modules.iter().copied().find(|module| {
        path_matches_hint(&module.path, project_root, &hint.owner_path)
            || path_matches_hint(&module.path, package_root, &hint.owner_path)
    })
}

fn path_matches_hint(path: &Path, root: &Path, hint_path: &Path) -> bool {
    if hint_path.is_absolute() {
        return path == hint_path;
    }
    path.strip_prefix(root)
        .is_ok_and(|relative_path| relative_path == hint_path)
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

fn task_kinds_for_responsibilities(
    responsibilities: &BTreeSet<RustOwnerResponsibility>,
    policy: &RustVerificationPolicy,
) -> BTreeSet<RustVerificationTaskKind> {
    responsibilities
        .iter()
        .flat_map(|responsibility| {
            policy
                .responsibility_task_overrides
                .get(responsibility)
                .cloned()
                .unwrap_or_else(|| default_task_kinds_for_responsibility(*responsibility))
        })
        .collect()
}

fn task_kinds_for_profile(
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
) -> BTreeSet<RustVerificationTaskKind> {
    hint.task_kinds
        .clone()
        .unwrap_or_else(|| task_kinds_for_responsibilities(&hint.responsibilities, policy))
}

fn default_task_kinds_for_responsibility(
    responsibility: RustOwnerResponsibility,
) -> BTreeSet<RustVerificationTaskKind> {
    match responsibility {
        RustOwnerResponsibility::PublicApi | RustOwnerResponsibility::LatencySensitive => {
            BTreeSet::from([RustVerificationTaskKind::Stress])
        }
        RustOwnerResponsibility::ExternalDependency
        | RustOwnerResponsibility::Persistence
        | RustOwnerResponsibility::AvailabilityCritical => {
            BTreeSet::from([RustVerificationTaskKind::Chaos])
        }
        RustOwnerResponsibility::SecurityBoundary => {
            BTreeSet::from([RustVerificationTaskKind::Security])
        }
        RustOwnerResponsibility::PureDomainLogic => BTreeSet::new(),
    }
}

fn profile_task_reason(
    kind: RustVerificationTaskKind,
    responsibilities: &BTreeSet<RustOwnerResponsibility>,
    uses_owner_task_override: bool,
) -> String {
    if uses_owner_task_override {
        return format!(
            "owner profile explicitly requests {} verification",
            kind.as_str()
        );
    }
    if responsibilities.iter().any(|responsibility| {
        default_task_kinds_for_responsibility(*responsibility).contains(&kind)
    }) {
        default_profile_task_reason(kind).to_string()
    } else {
        format!(
            "profile config maps responsibilities to {} verification",
            kind.as_str()
        )
    }
}

fn default_profile_task_reason(kind: RustVerificationTaskKind) -> &'static str {
    match kind {
        RustVerificationTaskKind::Stress => "profile declares public or latency-sensitive surface",
        RustVerificationTaskKind::Chaos => {
            "profile declares dependency, persistence, or availability responsibility"
        }
        RustVerificationTaskKind::Security => {
            "profile declares auth, authorization, secret, or trust-boundary logic"
        }
        RustVerificationTaskKind::Regression => {
            "profile config maps responsibilities to regression verification"
        }
        RustVerificationTaskKind::ResponsibilityReview => {
            "profile config maps responsibilities to responsibility review"
        }
    }
}

fn task_contract_for_profile(
    policy: &RustVerificationPolicy,
    hint: Option<&RustVerificationProfileHint>,
    kind: RustVerificationTaskKind,
) -> RustVerificationTaskContract {
    hint.and_then(|hint| hint.task_contract_overrides.get(&kind).cloned())
        .or_else(|| policy.task_contract_overrides.get(&kind).cloned())
        .unwrap_or_else(|| default_task_contract(kind))
}

fn default_task_contract(kind: RustVerificationTaskKind) -> RustVerificationTaskContract {
    match kind {
        RustVerificationTaskKind::Stress => RustVerificationTaskContract::new(
            RustVerificationPhase::AfterUnitTestsPass,
            "stress skill must report p50/p99/p999, load steps, and SLA result for this fingerprint",
            stress_requirements(),
        ),
        RustVerificationTaskKind::Chaos => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "chaos skill must report injected failures, degradation behavior, and recovery result for this fingerprint",
            chaos_requirements(),
        ),
        RustVerificationTaskKind::Security => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "security skill must report scanned attack classes and authorization-boundary result for this fingerprint",
            security_requirements(),
        ),
        RustVerificationTaskKind::Regression => RustVerificationTaskContract::new(
            RustVerificationPhase::ScheduledRegression,
            "regression skill must report source growth, dependency drift, and module-cycle status for this fingerprint",
            regression_requirements(),
        ),
        RustVerificationTaskKind::ResponsibilityReview => RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeVerification,
            "update the verification profile hint to match parser facts, or attach a complete waiver",
            responsibility_review_requirements(),
        ),
    }
}

fn stress_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("p50", "median latency under the chosen load step"),
        ("p99", "p99 latency under the chosen load step"),
        (
            "p999",
            "p999 latency when available or explicitly unsupported",
        ),
        (
            "load_steps",
            "pressure staircase and concurrency/request rates",
        ),
        ("sla_result", "whether the declared SLA was held or broken"),
    ])
}

fn chaos_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        (
            "injected_failures",
            "dependencies and failure modes injected",
        ),
        ("degradation", "observed degraded behavior during the fault"),
        (
            "recovery",
            "recovery signal and time after the fault is removed",
        ),
    ])
}

fn security_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("attack_classes", "common attack classes scanned"),
        (
            "authorization_boundary",
            "authorization or trust-boundary result",
        ),
        ("findings", "confirmed findings or explicit none result"),
    ])
}

fn regression_requirements() -> Vec<RustVerificationRequirement> {
    requirements([
        ("source_growth", "source growth or owner bloat trend"),
        (
            "dependency_drift",
            "owner dependency drift or fan-out change",
        ),
        ("module_cycles", "module or owner-cycle status"),
    ])
}

fn responsibility_review_requirements() -> Vec<RustVerificationRequirement> {
    requirements([(
        "profile_resolution",
        "updated responsibility hint or complete waiver rationale",
    )])
}

fn requirements<const N: usize>(
    requirements: [(&'static str, &'static str); N],
) -> Vec<RustVerificationRequirement> {
    requirements
        .into_iter()
        .map(|(key, description)| RustVerificationRequirement::new(key, description))
        .collect()
}
