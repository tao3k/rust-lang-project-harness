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

use super::api_path::{collect_api_path_baseline_tasks, collect_unmatched_api_path_baselines};
use super::profile::{
    hint_rationale_is_empty, profile_evidence, profile_task_reason, responsibility_labels,
    task_contract_for_profile, task_kind_labels, task_kinds_for_profile,
    task_kinds_for_responsibilities,
};
use super::task_builder::{
    ProfileReviewTaskSpec, VerificationTaskSpec, new_profile_review_task, new_skill_task,
    push_task, skill_descriptors_for_tasks,
};
use super::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReportObligation,
    RustVerificationTask, RustVerificationTaskKind,
};

struct ProfileConfigReviewTaskSpec<'a> {
    module: &'a RustReasoningModuleFacts,
    hint: &'a RustVerificationProfileHint,
    reason: &'static str,
    evidence: Vec<RustVerificationEvidence>,
}

struct VerificationTaskCollections<'a> {
    matched_profile_hints: &'a mut BTreeSet<usize>,
    matched_api_path_baselines: &'a mut BTreeSet<usize>,
    tasks: &'a mut BTreeMap<String, RustVerificationTask>,
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
    let mut matched_api_path_baselines = BTreeSet::new();
    {
        let mut collections = VerificationTaskCollections {
            matched_profile_hints: &mut matched_profile_hints,
            matched_api_path_baselines: &mut matched_api_path_baselines,
            tasks: &mut tasks,
        };
        for package_root in package_roots {
            collect_package_verification_tasks(
                project_root,
                &package_root,
                config,
                policy,
                &mut collections,
            );
        }
    }
    collect_unmatched_profile_hints(project_root, policy, &matched_profile_hints, &mut tasks);
    collect_unmatched_api_path_baselines(
        project_root,
        policy,
        &matched_api_path_baselines,
        &mut tasks,
    );
    let mut task_values = tasks.into_values().collect::<Vec<_>>();
    let mut plan = RustVerificationPlan {
        project_root: project_root.to_path_buf(),
        skill_descriptors: skill_descriptors_for_tasks(policy, &task_values),
        report_obligations: report_obligations_for_tasks(&task_values),
        tasks: std::mem::take(&mut task_values),
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

fn collect_package_verification_tasks(
    project_root: &Path,
    package_root: &Path,
    config: &RustHarnessConfig,
    policy: &RustVerificationPolicy,
    collections: &mut VerificationTaskCollections<'_>,
) {
    let scope = rust_project_harness_scope(
        package_root,
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
        &mut *collections.matched_profile_hints,
        &mut *collections.tasks,
    );
    collect_api_path_baseline_tasks(
        project_root,
        &reasoning_tree.package_root,
        &reasoning_tree.modules,
        policy,
        &mut *collections.matched_api_path_baselines,
        &mut *collections.tasks,
    );
    collect_regression_tasks(
        project_root,
        &reasoning_tree.package_root,
        &reasoning_tree.owner_branches,
        policy,
        &mut *collections.tasks,
    );
}

fn report_obligations_for_tasks(
    tasks: &[RustVerificationTask],
) -> Vec<RustVerificationReportObligation> {
    let active_tasks = tasks
        .iter()
        .filter(|task| task.is_active())
        .collect::<Vec<_>>();
    if active_tasks.is_empty() {
        return Vec::new();
    }

    let task_kinds = active_tasks
        .iter()
        .map(|task| task.kind)
        .collect::<BTreeSet<_>>();
    let task_fingerprints = active_tasks
        .iter()
        .map(|task| task.fingerprint.clone())
        .collect::<Vec<_>>();
    let mut obligations = vec![RustVerificationReportObligation::new(
        "verification_plan_json",
        "render_rust_verification_plan_json",
        "verification_plan.json",
        "persist active verification policy state so receipts, waivers, and task drift stay comparable",
        task_kinds,
        task_fingerprints,
    )];
    let configured_skill_tasks = active_tasks
        .iter()
        .filter(|task| task.skill_binding.is_some())
        .collect::<Vec<_>>();
    if !configured_skill_tasks.is_empty() {
        obligations.push(RustVerificationReportObligation::new(
            "task_index_json",
            "build_rust_verification_task_index + render_rust_verification_task_index_json",
            "task_index.json",
            "persist compact configured-skill task state for security, performance, stress, chaos, and regression",
            configured_skill_tasks
                .iter()
                .map(|task| task.kind)
                .collect::<BTreeSet<_>>(),
            configured_skill_tasks
                .iter()
                .map(|task| task.fingerprint.clone())
                .collect::<Vec<_>>(),
        ));
    }

    let performance_fingerprints = active_tasks
        .iter()
        .filter(|task| task.kind == RustVerificationTaskKind::Performance)
        .map(|task| task.fingerprint.clone())
        .collect::<Vec<_>>();
    if !performance_fingerprints.is_empty() {
        obligations.push(RustVerificationReportObligation::new(
            "performance_index_json",
            "build_rust_verification_performance_index + render_rust_verification_performance_index_json",
            "performance_index.json",
            "persist Rust performance state for benchmark, receipt, and missing-evidence metrics",
            [RustVerificationTaskKind::Performance],
            performance_fingerprints,
        ));
    }

    obligations
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
        collect_profile_config_review_tasks(
            project_root,
            package_root,
            module,
            hint,
            policy,
            tasks,
        );
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

fn collect_profile_config_review_tasks(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    if hint.responsibilities.is_empty() {
        push_profile_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ProfileConfigReviewTaskSpec {
                module,
                hint,
                reason: "profile hint declares no responsibilities for owner",
                evidence: vec![RustVerificationEvidence::new(
                    "profile",
                    "responsibilities=<none>",
                )],
            },
        );
    }

    let effective_task_kinds = task_kinds_for_profile(hint, policy);
    if let Some(owner_task_kinds) = &hint.task_kinds {
        let derived_task_kinds = task_kinds_for_responsibilities(&hint.responsibilities, policy);
        if owner_task_kinds != &derived_task_kinds && hint_rationale_is_empty(hint) {
            push_profile_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ProfileConfigReviewTaskSpec {
                    module,
                    hint,
                    reason: "owner-local verification override needs compact rationale",
                    evidence: vec![
                        RustVerificationEvidence::new(
                            "profile",
                            responsibility_labels(&hint.responsibilities),
                        ),
                        RustVerificationEvidence::new(
                            "derived",
                            task_kind_labels(&derived_task_kinds),
                        ),
                        RustVerificationEvidence::new(
                            "configured",
                            task_kind_labels(owner_task_kinds),
                        ),
                    ],
                },
            );
        }

        let disabled_task_kinds = owner_task_kinds
            .intersection(&policy.disabled_task_kinds)
            .copied()
            .collect::<BTreeSet<_>>();
        if !disabled_task_kinds.is_empty() {
            push_profile_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ProfileConfigReviewTaskSpec {
                    module,
                    hint,
                    reason: "owner-local verification override references disabled task kind",
                    evidence: vec![
                        RustVerificationEvidence::new(
                            "configured",
                            task_kind_labels(owner_task_kinds),
                        ),
                        RustVerificationEvidence::new(
                            "disabled",
                            task_kind_labels(&disabled_task_kinds),
                        ),
                    ],
                },
            );
        }
    }

    let unused_contract_kinds = hint
        .task_contract_overrides
        .keys()
        .filter(|kind| !effective_task_kinds.contains(kind))
        .copied()
        .collect::<BTreeSet<_>>();
    if !unused_contract_kinds.is_empty() {
        push_profile_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ProfileConfigReviewTaskSpec {
                module,
                hint,
                reason: "owner-local task contract is not used by effective task kinds",
                evidence: vec![
                    RustVerificationEvidence::new(
                        "effective",
                        task_kind_labels(&effective_task_kinds),
                    ),
                    RustVerificationEvidence::new(
                        "unused_contracts",
                        task_kind_labels(&unused_contract_kinds),
                    ),
                ],
            },
        );
    }
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
                    evidence: profile_evidence(hint),
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

fn push_profile_config_review_task(
    project_root: &Path,
    package_root: &Path,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
    spec: ProfileConfigReviewTaskSpec<'_>,
) {
    push_task(
        tasks,
        policy,
        new_profile_review_task(
            project_root,
            package_root,
            ProfileReviewTaskSpec {
                owner_path: spec.module.path.clone(),
                owner_namespace: spec.module.source_path.namespace_components.clone(),
                reason: spec.reason,
                evidence: spec.evidence,
                hint: Some(spec.hint),
            },
            policy,
        ),
    );
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

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}
