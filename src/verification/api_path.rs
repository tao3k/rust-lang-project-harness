//! API path-level verification task planning.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::parser::RustReasoningModuleFacts;

use super::profile::{
    api_path_baseline_evidence, api_path_rationale_is_empty, api_path_task_reason,
    responsibility_labels, task_contract_for_api_path_baseline, task_kind_labels,
    task_kinds_for_api_path_baseline, task_kinds_for_responsibilities,
};
use super::task_builder::{
    ProfileReviewTaskSpec, VerificationTaskSpec, new_profile_review_task, new_skill_task, push_task,
};
use super::{
    RustVerificationApiPathBaseline, RustVerificationEvidence, RustVerificationPolicy,
    RustVerificationTask,
};

struct ApiPathConfigReviewTaskSpec<'a> {
    module: &'a RustReasoningModuleFacts,
    reason: &'static str,
    evidence: Vec<RustVerificationEvidence>,
}

pub(super) fn collect_api_path_baseline_tasks(
    project_root: &Path,
    package_root: &Path,
    modules: &[RustReasoningModuleFacts],
    policy: &RustVerificationPolicy,
    matched_api_path_baselines: &mut BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let source_modules = modules
        .iter()
        .filter(|module| module.is_source_module)
        .collect::<Vec<_>>();
    for (baseline_index, baseline) in policy.api_path_baselines.iter().enumerate() {
        let Some(module) = matching_api_path_baseline_module(
            project_root,
            package_root,
            &source_modules,
            baseline,
        ) else {
            continue;
        };
        matched_api_path_baselines.insert(baseline_index);
        collect_api_path_config_review_tasks(
            project_root,
            package_root,
            module,
            baseline,
            policy,
            tasks,
        );
        collect_skill_tasks_from_api_path_baseline(
            project_root,
            package_root,
            module,
            baseline,
            policy,
            tasks,
        );
    }
}

pub(super) fn collect_unmatched_api_path_baselines(
    project_root: &Path,
    policy: &RustVerificationPolicy,
    matched_api_path_baselines: &BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for (baseline_index, baseline) in policy.api_path_baselines.iter().enumerate() {
        if matched_api_path_baselines.contains(&baseline_index) {
            continue;
        }
        let owner_path = if baseline.owner_path.is_absolute() {
            baseline.owner_path.clone()
        } else {
            project_root.join(&baseline.owner_path)
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
                    reason: "API path baseline target is not a parser-known Rust source module",
                    evidence: api_path_config_review_evidence(
                        baseline,
                        vec![
                            RustVerificationEvidence::new(
                                "responsibilities",
                                responsibility_labels(&baseline.responsibilities),
                            ),
                            RustVerificationEvidence::new(
                                "task_kinds",
                                task_kind_labels(&task_kinds_for_api_path_baseline(
                                    baseline, policy,
                                )),
                            ),
                        ],
                    ),
                    hint: None,
                },
                policy,
            ),
        );
    }
}

fn collect_api_path_config_review_tasks(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    baseline: &RustVerificationApiPathBaseline,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    if baseline.responsibilities.is_empty() {
        push_api_path_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ApiPathConfigReviewTaskSpec {
                module,
                reason: "API path baseline declares no responsibilities",
                evidence: api_path_config_review_evidence(
                    baseline,
                    vec![RustVerificationEvidence::new(
                        "profile",
                        "responsibilities=<none>",
                    )],
                ),
            },
        );
    }

    let effective_task_kinds = task_kinds_for_api_path_baseline(baseline, policy);
    if let Some(path_task_kinds) = &baseline.task_kinds {
        let derived_task_kinds =
            task_kinds_for_responsibilities(&baseline.responsibilities, policy);
        if path_task_kinds != &derived_task_kinds && api_path_rationale_is_empty(baseline) {
            push_api_path_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ApiPathConfigReviewTaskSpec {
                    module,
                    reason: "API path verification override needs compact rationale",
                    evidence: api_path_config_review_evidence(
                        baseline,
                        vec![
                            RustVerificationEvidence::new(
                                "derived",
                                task_kind_labels(&derived_task_kinds),
                            ),
                            RustVerificationEvidence::new(
                                "configured",
                                task_kind_labels(path_task_kinds),
                            ),
                        ],
                    ),
                },
            );
        }

        let disabled_task_kinds = path_task_kinds
            .intersection(&policy.disabled_task_kinds)
            .copied()
            .collect::<BTreeSet<_>>();
        if !disabled_task_kinds.is_empty() {
            push_api_path_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ApiPathConfigReviewTaskSpec {
                    module,
                    reason: "API path verification override references disabled task kind",
                    evidence: api_path_config_review_evidence(
                        baseline,
                        vec![RustVerificationEvidence::new(
                            "disabled",
                            task_kind_labels(&disabled_task_kinds),
                        )],
                    ),
                },
            );
        }
    }

    let unused_contract_kinds = baseline
        .task_contract_overrides
        .keys()
        .filter(|kind| !effective_task_kinds.contains(kind))
        .copied()
        .collect::<BTreeSet<_>>();
    if !unused_contract_kinds.is_empty() {
        push_api_path_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ApiPathConfigReviewTaskSpec {
                module,
                reason: "API path task contract is not used by effective task kinds",
                evidence: api_path_config_review_evidence(
                    baseline,
                    vec![
                        RustVerificationEvidence::new(
                            "effective",
                            task_kind_labels(&effective_task_kinds),
                        ),
                        RustVerificationEvidence::new(
                            "unused_contracts",
                            task_kind_labels(&unused_contract_kinds),
                        ),
                    ],
                ),
            },
        );
    }
}

fn collect_skill_tasks_from_api_path_baseline(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    baseline: &RustVerificationApiPathBaseline,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let uses_path_task_override = baseline.task_kinds.is_some();
    for kind in task_kinds_for_api_path_baseline(baseline, policy) {
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
                    reason: api_path_task_reason(kind, baseline, uses_path_task_override),
                    contract: task_contract_for_api_path_baseline(policy, baseline, kind),
                    evidence: api_path_baseline_evidence(baseline),
                },
                policy,
            ),
        );
    }
}

fn push_api_path_config_review_task(
    project_root: &Path,
    package_root: &Path,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
    spec: ApiPathConfigReviewTaskSpec<'_>,
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
                hint: None,
            },
            policy,
        ),
    );
}

fn api_path_config_review_evidence(
    baseline: &RustVerificationApiPathBaseline,
    extras: Vec<RustVerificationEvidence>,
) -> Vec<RustVerificationEvidence> {
    let mut evidence = api_path_baseline_evidence(baseline);
    evidence.extend(extras);
    evidence
}

fn matching_api_path_baseline_module<'a>(
    project_root: &Path,
    package_root: &Path,
    modules: &[&'a RustReasoningModuleFacts],
    baseline: &RustVerificationApiPathBaseline,
) -> Option<&'a RustReasoningModuleFacts> {
    modules.iter().copied().find(|module| {
        path_matches_baseline(&module.path, project_root, &baseline.owner_path)
            || path_matches_baseline(&module.path, package_root, &baseline.owner_path)
    })
}

fn path_matches_baseline(path: &Path, root: &Path, baseline_path: &Path) -> bool {
    if baseline_path.is_absolute() {
        return path == baseline_path;
    }
    path.strip_prefix(root)
        .is_ok_and(|relative_path| relative_path == baseline_path)
}
