use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, ParsedRustModule, RustReasoningModuleFacts,
    RustReasoningOwnerBranchFacts, RustReasoningOwnerBranchRole, RustUseImportRootKind,
};

use super::model::{RustVerificationProfileCandidate, RustVerificationProfileCandidateState};
use super::taxonomy::standard_import_responsibilities;
use crate::verification::profile::task_kinds_for_responsibilities;
use crate::verification::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPolicy,
    RustVerificationProfileHint,
};

#[derive(Debug, Default)]
struct ProfileSignals {
    owner_modules: usize,
    public_items: usize,
    public_exports: usize,
    public_functions: usize,
    owner_deps: usize,
    child_modules: usize,
    dependency_roots: BTreeSet<String>,
    configured_dependency_roots: BTreeSet<String>,
    unconfigured_dependency_roots: BTreeSet<String>,
    external_roots: BTreeSet<String>,
    network_roots: BTreeSet<String>,
    persistence_roots: BTreeSet<String>,
    security_roots: BTreeSet<String>,
    performance_roots: BTreeSet<String>,
}

pub(super) struct PackageCandidateInput<'a> {
    pub(super) project_root: &'a Path,
    pub(super) package_root: &'a Path,
    pub(super) modules: &'a [RustReasoningModuleFacts],
    pub(super) branches: &'a [RustReasoningOwnerBranchFacts],
    pub(super) parsed_modules: &'a [ParsedRustModule],
    pub(super) cargo_dependencies: &'a [CargoDependencyFacts],
    pub(super) policy: &'a RustVerificationPolicy,
}

pub(super) fn collect_package_candidates(
    input: PackageCandidateInput<'_>,
    candidates: &mut Vec<RustVerificationProfileCandidate>,
) {
    let parsed_by_path = input
        .parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let branches_by_path = input
        .branches
        .iter()
        .map(|branch| (branch.path.clone(), branch))
        .collect::<BTreeMap<_, _>>();
    let profiled_branches = input
        .branches
        .iter()
        .filter(|branch| branch_is_profile_owner(branch))
        .collect::<Vec<_>>();
    let mut covered_leaf_paths = BTreeSet::new();
    let mut branch_modules_by_path = BTreeMap::<PathBuf, Vec<&RustReasoningModuleFacts>>::new();
    for module in input
        .modules
        .iter()
        .filter(|module| module.is_source_module)
    {
        if let Some(branch) = nearest_profile_branch(module, &profiled_branches) {
            if module.path != branch.path {
                covered_leaf_paths.insert(module.path.clone());
            }
            branch_modules_by_path
                .entry(branch.path.clone())
                .or_default()
                .push(module);
        }
    }
    for branch in &profiled_branches {
        let Some(branch_module) = input
            .modules
            .iter()
            .find(|module| module.path == branch.path)
        else {
            continue;
        };
        let Some(branch_modules) = branch_modules_by_path.get(&branch.path) else {
            continue;
        };
        let signals = aggregate_profile_signals(
            branch_modules,
            Some(branch),
            &parsed_by_path,
            input.cargo_dependencies,
            input.policy,
        );
        push_profile_candidate(
            input.project_root,
            input.package_root,
            branch_module,
            Some(branch),
            signals,
            input.policy,
            candidates,
        );
    }
    for module in input
        .modules
        .iter()
        .filter(|module| module.is_source_module)
        .filter(|module| !covered_leaf_paths.contains(&module.path))
    {
        let branch = branches_by_path.get(&module.path).copied();
        if branch.is_some_and(branch_is_profile_owner) {
            continue;
        }
        let signals = aggregate_profile_signals(
            &[module],
            branch,
            &parsed_by_path,
            input.cargo_dependencies,
            input.policy,
        );
        push_profile_candidate(
            input.project_root,
            input.package_root,
            module,
            branch,
            signals,
            input.policy,
            candidates,
        );
    }
}

fn push_profile_candidate(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    branch: Option<&RustReasoningOwnerBranchFacts>,
    signals: ProfileSignals,
    policy: &RustVerificationPolicy,
    candidates: &mut Vec<RustVerificationProfileCandidate>,
) {
    let responsibilities = suggested_responsibilities(module, branch, &signals);
    if responsibilities.is_empty() {
        return;
    }
    let matching_hint = matching_profile_hint(project_root, package_root, &module.path, policy);
    let configured_responsibilities = matching_hint
        .map(|hint| hint.responsibilities.clone())
        .unwrap_or_default();
    let state = profile_candidate_state(matching_hint, &responsibilities);
    candidates.push(RustVerificationProfileCandidate {
        package_root: package_root.to_path_buf(),
        owner_path: module.path.clone(),
        hint_path: recommended_hint_path(project_root, package_root, &module.path),
        owner_namespace: module.source_path.namespace_components.clone(),
        state,
        suggested_task_kinds: task_kinds_for_responsibilities(&responsibilities, policy),
        suggested_responsibilities: responsibilities,
        configured_responsibilities,
        evidence: profile_evidence(&signals),
    });
}

fn aggregate_profile_signals(
    modules: &[&RustReasoningModuleFacts],
    branch: Option<&RustReasoningOwnerBranchFacts>,
    parsed_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    cargo_dependencies: &[CargoDependencyFacts],
    policy: &RustVerificationPolicy,
) -> ProfileSignals {
    let mut signals = ProfileSignals {
        owner_modules: modules.len(),
        child_modules: branch.map_or(0, |branch| branch.declared_child_edges.len()),
        ..ProfileSignals::default()
    };
    for module in modules {
        let Some(parsed_module) = parsed_by_path.get(&module.path) else {
            continue;
        };
        merge_profile_signals(
            &mut signals,
            module,
            parsed_module,
            cargo_dependencies,
            policy,
        );
    }
    signals
}

fn merge_profile_signals(
    signals: &mut ProfileSignals,
    module: &RustReasoningModuleFacts,
    parsed_module: &ParsedRustModule,
    cargo_dependencies: &[CargoDependencyFacts],
    policy: &RustVerificationPolicy,
) {
    signals.owner_deps += module
        .import_summary
        .local_owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .count();
    for item in &parsed_module.syntax_facts.top_level_items {
        if item.is_public && item.kind != "mod" {
            signals.public_items += 1;
        }
        if item.is_public_use {
            signals.public_exports += 1;
        }
        if item.is_public && item.kind == "fn" {
            signals.public_functions += 1;
        }
    }
    collect_import_signals(parsed_module, signals, cargo_dependencies, policy);
}

fn collect_import_signals(
    parsed_module: &ParsedRustModule,
    signals: &mut ProfileSignals,
    cargo_dependencies: &[CargoDependencyFacts],
    policy: &RustVerificationPolicy,
) {
    for use_statement in &parsed_module.syntax_facts.use_statements {
        if use_statement.context.is_inside_cfg_test_module {
            continue;
        }
        for import in &use_statement.imports {
            if !matches!(
                import.root_kind,
                RustUseImportRootKind::External | RustUseImportRootKind::Absolute
            ) {
                continue;
            }
            add_import_signal(&import.segments, signals, cargo_dependencies, policy);
        }
    }
}

fn add_import_signal(
    segments: &[String],
    signals: &mut ProfileSignals,
    cargo_dependencies: &[CargoDependencyFacts],
    policy: &RustVerificationPolicy,
) {
    if segments.is_empty() {
        return;
    }
    let label = compact_import_label(segments);
    let mut responsibilities = standard_import_responsibilities(segments);
    if let Some(dependency) = matching_cargo_dependency(segments, cargo_dependencies) {
        let dependency_label = compact_dependency_label(dependency);
        signals.dependency_roots.insert(dependency_label.clone());
        let configured_responsibilities =
            configured_dependency_responsibilities(dependency, policy);
        if configured_responsibilities.is_empty() {
            signals
                .unconfigured_dependency_roots
                .insert(dependency_label);
        } else {
            signals.configured_dependency_roots.insert(dependency_label);
        }
        responsibilities.extend(configured_responsibilities);
    }
    for responsibility in responsibilities {
        add_import_responsibility(signals, responsibility, &label);
    }
}

fn matching_cargo_dependency<'a>(
    segments: &[String],
    cargo_dependencies: &'a [CargoDependencyFacts],
) -> Option<&'a CargoDependencyFacts> {
    let root = segments.first()?;
    cargo_dependencies
        .iter()
        .find(|dependency| dependency.import_name == *root)
}

fn configured_dependency_responsibilities(
    dependency: &CargoDependencyFacts,
    policy: &RustVerificationPolicy,
) -> BTreeSet<RustOwnerResponsibility> {
    policy
        .dependency_signals
        .iter()
        .filter(|signal| {
            signal.matches_dependency(
                &dependency.dependency_key,
                &dependency.import_name,
                &dependency.package_name,
            )
        })
        .flat_map(|signal| signal.responsibilities.iter().copied())
        .collect()
}

fn compact_dependency_label(dependency: &CargoDependencyFacts) -> String {
    let base = if dependency.import_name == dependency.package_name {
        dependency.import_name.clone()
    } else {
        format!("{}->{}", dependency.import_name, dependency.package_name)
    };
    let mut qualifiers = Vec::new();
    match dependency.kind {
        CargoDependencyKind::Normal => {}
        CargoDependencyKind::Dev => qualifiers.push("dev".into()),
        CargoDependencyKind::Build => {
            qualifiers.push("build".into());
        }
    }
    if let Some(target) = &dependency.target {
        qualifiers.push(format!("target={target}"));
    }
    if dependency.optional {
        qualifiers.push("optional".into());
    }
    if !dependency.features.is_empty() {
        qualifiers.push(format!("features={}", dependency.features.join("+")));
    }
    if qualifiers.is_empty() {
        base
    } else {
        format!("{base}({})", qualifiers.join(","))
    }
}

fn add_import_responsibility(
    signals: &mut ProfileSignals,
    responsibility: RustOwnerResponsibility,
    label: &str,
) {
    match responsibility {
        RustOwnerResponsibility::ExternalDependency => {
            signals.external_roots.insert(label.to_string());
        }
        RustOwnerResponsibility::Persistence => {
            signals.persistence_roots.insert(label.to_string());
        }
        RustOwnerResponsibility::SecurityBoundary => {
            signals.security_roots.insert(label.to_string());
        }
        RustOwnerResponsibility::LatencySensitive => {
            signals.performance_roots.insert(label.to_string());
        }
        RustOwnerResponsibility::AvailabilityCritical => {
            signals.network_roots.insert(label.to_string());
        }
        RustOwnerResponsibility::PublicApi | RustOwnerResponsibility::PureDomainLogic => {}
    }
}

fn compact_import_label(segments: &[String]) -> String {
    segments
        .iter()
        .take(2)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("::")
}

fn suggested_responsibilities(
    module: &RustReasoningModuleFacts,
    branch: Option<&RustReasoningOwnerBranchFacts>,
    signals: &ProfileSignals,
) -> BTreeSet<RustOwnerResponsibility> {
    let mut responsibilities = BTreeSet::new();
    if signals.public_functions > 0
        || signals.public_items > 0
        || (signals.public_exports > 0 && !is_crate_root_facade(module))
        || branch.is_some_and(branch_is_public_surface)
    {
        responsibilities.insert(RustOwnerResponsibility::PublicApi);
    }
    if !signals.external_roots.is_empty()
        || !signals.network_roots.is_empty()
        || !signals.persistence_roots.is_empty()
        || signals.owner_deps >= 3
    {
        responsibilities.insert(RustOwnerResponsibility::ExternalDependency);
    }
    if !signals.persistence_roots.is_empty() {
        responsibilities.insert(RustOwnerResponsibility::Persistence);
    }
    if !signals.security_roots.is_empty() {
        responsibilities.insert(RustOwnerResponsibility::SecurityBoundary);
    }
    if !signals.performance_roots.is_empty() {
        responsibilities.insert(RustOwnerResponsibility::LatencySensitive);
    }
    if !signals.network_roots.is_empty() {
        responsibilities.insert(RustOwnerResponsibility::AvailabilityCritical);
    }
    responsibilities
}

fn is_crate_root_facade(module: &RustReasoningModuleFacts) -> bool {
    module.is_module_tree_root && module.source_path.is_crate_facade
}

fn nearest_profile_branch<'a>(
    module: &RustReasoningModuleFacts,
    branches: &[&'a RustReasoningOwnerBranchFacts],
) -> Option<&'a RustReasoningOwnerBranchFacts> {
    branches
        .iter()
        .copied()
        .filter(|branch| {
            namespace_has_prefix(
                &module.source_path.namespace_components,
                &branch.owner_namespace,
            )
        })
        .max_by_key(|branch| branch.owner_namespace.len())
}

fn branch_is_profile_owner(branch: &RustReasoningOwnerBranchFacts) -> bool {
    !branch.roles.contains(&RustReasoningOwnerBranchRole::Root)
        || branch.roles.contains(&RustReasoningOwnerBranchRole::Binary)
        || branch
            .roles
            .contains(&RustReasoningOwnerBranchRole::PackageEntrypoint)
}

fn branch_is_public_surface(branch: &RustReasoningOwnerBranchFacts) -> bool {
    if branch.roles.contains(&RustReasoningOwnerBranchRole::Root) {
        return false;
    }
    branch.roles.iter().any(|role| {
        matches!(
            role,
            RustReasoningOwnerBranchRole::Facade
                | RustReasoningOwnerBranchRole::Interface
                | RustReasoningOwnerBranchRole::Binary
                | RustReasoningOwnerBranchRole::PackageEntrypoint
        )
    })
}

fn profile_evidence(signals: &ProfileSignals) -> Vec<RustVerificationEvidence> {
    let mut evidence = Vec::new();
    if signals.owner_modules > 1 {
        push_usize_evidence(&mut evidence, "owner_modules", signals.owner_modules);
    }
    push_usize_evidence(&mut evidence, "public_items", signals.public_items);
    push_usize_evidence(&mut evidence, "public_exports", signals.public_exports);
    push_usize_evidence(&mut evidence, "public_fns", signals.public_functions);
    push_usize_evidence(&mut evidence, "owner_deps", signals.owner_deps);
    push_usize_evidence(&mut evidence, "child_modules", signals.child_modules);
    push_set_evidence(&mut evidence, "dependency_roots", &signals.dependency_roots);
    push_set_evidence(
        &mut evidence,
        "configured_dependency_roots",
        &signals.configured_dependency_roots,
    );
    push_set_evidence(
        &mut evidence,
        "unconfigured_dependency_roots",
        &signals.unconfigured_dependency_roots,
    );
    push_set_evidence(&mut evidence, "external_roots", &signals.external_roots);
    push_set_evidence(&mut evidence, "network_roots", &signals.network_roots);
    push_set_evidence(
        &mut evidence,
        "persistence_roots",
        &signals.persistence_roots,
    );
    push_set_evidence(&mut evidence, "security_roots", &signals.security_roots);
    push_set_evidence(
        &mut evidence,
        "performance_roots",
        &signals.performance_roots,
    );
    evidence
}

fn push_usize_evidence(evidence: &mut Vec<RustVerificationEvidence>, label: &str, value: usize) {
    if value > 0 {
        evidence.push(RustVerificationEvidence::new(label, value.to_string()));
    }
}

fn push_set_evidence(
    evidence: &mut Vec<RustVerificationEvidence>,
    label: &str,
    values: &BTreeSet<String>,
) {
    if !values.is_empty() {
        evidence.push(RustVerificationEvidence::new(
            label,
            values.iter().cloned().collect::<Vec<_>>().join(","),
        ));
    }
}

fn profile_candidate_state(
    hint: Option<&RustVerificationProfileHint>,
    suggested: &BTreeSet<RustOwnerResponsibility>,
) -> RustVerificationProfileCandidateState {
    let Some(hint) = hint else {
        return RustVerificationProfileCandidateState::MissingProfile;
    };
    if suggested.is_subset(&hint.responsibilities) {
        RustVerificationProfileCandidateState::Configured
    } else {
        RustVerificationProfileCandidateState::ProfileDrift
    }
}

fn matching_profile_hint<'a>(
    project_root: &Path,
    package_root: &Path,
    owner_path: &Path,
    policy: &'a RustVerificationPolicy,
) -> Option<&'a RustVerificationProfileHint> {
    policy.profile_hints.iter().find(|hint| {
        path_matches_hint(owner_path, project_root, &hint.owner_path)
            || path_matches_hint(owner_path, package_root, &hint.owner_path)
    })
}

fn path_matches_hint(path: &Path, root: &Path, hint_path: &Path) -> bool {
    if hint_path.is_absolute() {
        return path == hint_path;
    }
    path.strip_prefix(root)
        .is_ok_and(|relative_path| relative_path == hint_path)
}

fn recommended_hint_path(project_root: &Path, package_root: &Path, owner_path: &Path) -> PathBuf {
    owner_path
        .strip_prefix(project_root)
        .or_else(|_| owner_path.strip_prefix(package_root))
        .map_or_else(|_| owner_path.to_path_buf(), Path::to_path_buf)
}

fn namespace_has_prefix(namespace: &[String], prefix: &[String]) -> bool {
    namespace.starts_with(prefix)
}
