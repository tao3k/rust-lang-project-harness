use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, RustReasoningImportFacts, RustReasoningModuleFacts,
    RustReasoningOwnerBranchFacts, RustReasoningOwnerBranchRole,
};

use super::model::{RustVerificationProfileCandidate, RustVerificationProfileCandidateState};
use super::taxonomy::standard_import_responsibilities;
use crate::verification::profile::task_kinds_for_responsibilities;
use crate::verification::{
    RustOwnerResponsibility, RustVerificationDependencySignal, RustVerificationEvidence,
    RustVerificationPolicy, RustVerificationProfileHint,
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

struct ProfileHintLookup<'a> {
    absolute: BTreeMap<PathBuf, &'a RustVerificationProfileHint>,
    relative: BTreeMap<PathBuf, &'a RustVerificationProfileHint>,
}

impl<'a> ProfileHintLookup<'a> {
    fn new(hints: &'a [RustVerificationProfileHint]) -> Self {
        let mut lookup = Self {
            absolute: BTreeMap::new(),
            relative: BTreeMap::new(),
        };
        for hint in hints {
            if hint.owner_path.is_absolute() {
                lookup
                    .absolute
                    .entry(hint.owner_path.clone())
                    .or_insert(hint);
            } else {
                lookup
                    .relative
                    .entry(hint.owner_path.clone())
                    .or_insert(hint);
            }
        }
        lookup
    }

    fn get(
        &self,
        project_root: &Path,
        package_root: &Path,
        owner_path: &Path,
    ) -> Option<&'a RustVerificationProfileHint> {
        if let Some(hint) = self.absolute.get(owner_path).copied() {
            return Some(hint);
        }
        owner_path
            .strip_prefix(project_root)
            .ok()
            .and_then(|relative_path| self.relative.get(relative_path).copied())
            .or_else(|| {
                owner_path
                    .strip_prefix(package_root)
                    .ok()
                    .and_then(|relative_path| self.relative.get(relative_path).copied())
            })
    }
}

struct DependencySignalLookup<'a> {
    cargo_by_import_name: BTreeMap<String, &'a CargoDependencyFacts>,
    responsibilities_by_dependency: BTreeMap<String, BTreeSet<RustOwnerResponsibility>>,
}

impl<'a> DependencySignalLookup<'a> {
    fn new(
        cargo_dependencies: &'a [CargoDependencyFacts],
        dependency_signals: &[RustVerificationDependencySignal],
    ) -> Self {
        let cargo_by_import_name = cargo_dependencies
            .iter()
            .map(|dependency| (dependency.import_name.clone(), dependency))
            .collect::<BTreeMap<_, _>>();
        let mut responsibilities_by_dependency = BTreeMap::new();
        for signal in dependency_signals {
            responsibilities_by_dependency
                .entry(signal.dependency.clone())
                .or_insert_with(BTreeSet::new)
                .extend(signal.responsibilities.iter().copied());
        }
        Self {
            cargo_by_import_name,
            responsibilities_by_dependency,
        }
    }

    fn matching_cargo_dependency(&self, segments: &[String]) -> Option<&'a CargoDependencyFacts> {
        let root = segments.first()?;
        self.cargo_by_import_name.get(root).copied()
    }

    fn configured_dependency_responsibilities(
        &self,
        dependency: &CargoDependencyFacts,
    ) -> BTreeSet<RustOwnerResponsibility> {
        let mut responsibilities = BTreeSet::new();
        for key in [
            dependency.dependency_key.as_str(),
            dependency.import_name.as_str(),
            dependency.package_name.as_str(),
        ] {
            if let Some(configured) = self.responsibilities_by_dependency.get(key) {
                responsibilities.extend(configured.iter().copied());
            }
        }
        responsibilities
    }
}

struct ProfileBranchLookup<'a> {
    branch_by_namespace: BTreeMap<Vec<String>, &'a RustReasoningOwnerBranchFacts>,
}

impl<'a> ProfileBranchLookup<'a> {
    fn new(branches: impl IntoIterator<Item = &'a RustReasoningOwnerBranchFacts>) -> Self {
        let mut branch_by_namespace = BTreeMap::new();
        for branch in branches {
            branch_by_namespace
                .entry(branch.owner_namespace.clone())
                .or_insert(branch);
        }
        Self {
            branch_by_namespace,
        }
    }

    fn nearest_profile_branch(
        &self,
        module: &RustReasoningModuleFacts,
    ) -> Option<&'a RustReasoningOwnerBranchFacts> {
        let namespace = module.source_path.namespace_components.as_slice();
        for prefix_len in (0..=namespace.len()).rev() {
            if let Some(branch) = self.branch_by_namespace.get(&namespace[..prefix_len]) {
                return Some(branch);
            }
        }
        None
    }
}

struct ProfileCandidatePushContext<'input, 'lookup, 'candidates> {
    project_root: &'input Path,
    package_root: &'input Path,
    hint_lookup: &'lookup ProfileHintLookup<'input>,
    policy: &'input RustVerificationPolicy,
    candidates: &'candidates mut Vec<RustVerificationProfileCandidate>,
}

pub(super) struct PackageCandidateInput<'a> {
    pub(super) project_root: &'a Path,
    pub(super) package_root: &'a Path,
    pub(super) modules: &'a [RustReasoningModuleFacts],
    pub(super) branches: &'a [RustReasoningOwnerBranchFacts],
    pub(super) cargo_dependencies: &'a [CargoDependencyFacts],
    pub(super) policy: &'a RustVerificationPolicy,
}

pub(super) fn collect_package_candidates(
    input: PackageCandidateInput<'_>,
    candidates: &mut Vec<RustVerificationProfileCandidate>,
) {
    let modules_by_path = input
        .modules
        .iter()
        .map(|module| (module.path.clone(), module))
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
    let branch_lookup = ProfileBranchLookup::new(profiled_branches.iter().copied());
    let hint_lookup = ProfileHintLookup::new(&input.policy.profile_hints);
    let dependency_lookup =
        DependencySignalLookup::new(input.cargo_dependencies, &input.policy.dependency_signals);
    let mut push_context = ProfileCandidatePushContext {
        project_root: input.project_root,
        package_root: input.package_root,
        hint_lookup: &hint_lookup,
        policy: input.policy,
        candidates,
    };
    let mut covered_leaf_paths = BTreeSet::new();
    let mut branch_modules_by_path = BTreeMap::<PathBuf, Vec<&RustReasoningModuleFacts>>::new();
    for module in input
        .modules
        .iter()
        .filter(|module| module.is_source_module)
    {
        if let Some(branch) = branch_lookup.nearest_profile_branch(module) {
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
        let Some(branch_module) = modules_by_path.get(&branch.path).copied() else {
            continue;
        };
        let Some(branch_modules) = branch_modules_by_path.get(&branch.path) else {
            continue;
        };
        let signals = aggregate_profile_signals(branch_modules, Some(branch), &dependency_lookup);
        push_profile_candidate(&mut push_context, branch_module, Some(branch), signals);
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
        let signals = aggregate_profile_signals(&[module], branch, &dependency_lookup);
        push_profile_candidate(&mut push_context, module, branch, signals);
    }
}

fn push_profile_candidate(
    context: &mut ProfileCandidatePushContext<'_, '_, '_>,
    module: &RustReasoningModuleFacts,
    branch: Option<&RustReasoningOwnerBranchFacts>,
    signals: ProfileSignals,
) {
    let responsibilities = suggested_responsibilities(module, branch, &signals);
    if responsibilities.is_empty() {
        return;
    }
    let matching_hint =
        context
            .hint_lookup
            .get(context.project_root, context.package_root, &module.path);
    let configured_responsibilities = matching_hint
        .map(|hint| hint.responsibilities.clone())
        .unwrap_or_default();
    let state = profile_candidate_state(matching_hint, &responsibilities);
    context.candidates.push(RustVerificationProfileCandidate {
        package_root: context.package_root.to_path_buf(),
        owner_path: module.path.clone(),
        hint_path: recommended_hint_path(context.project_root, context.package_root, &module.path),
        owner_namespace: module.source_path.namespace_components.clone(),
        state,
        suggested_task_kinds: task_kinds_for_responsibilities(&responsibilities, context.policy),
        suggested_responsibilities: responsibilities,
        configured_responsibilities,
        evidence: profile_evidence(&signals),
    });
}

fn aggregate_profile_signals(
    modules: &[&RustReasoningModuleFacts],
    branch: Option<&RustReasoningOwnerBranchFacts>,
    dependency_lookup: &DependencySignalLookup<'_>,
) -> ProfileSignals {
    let mut signals = ProfileSignals {
        owner_modules: modules.len(),
        child_modules: branch.map_or(0, |branch| branch.declared_child_edges.len()),
        ..ProfileSignals::default()
    };
    for module in modules {
        merge_profile_signals(&mut signals, module, dependency_lookup);
    }
    signals
}

fn merge_profile_signals(
    signals: &mut ProfileSignals,
    module: &RustReasoningModuleFacts,
    dependency_lookup: &DependencySignalLookup<'_>,
) {
    signals.owner_deps += module
        .import_summary
        .local_owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .count();
    signals.public_items += module.public_api_summary.public_items;
    signals.public_exports += module.public_api_summary.public_exports;
    signals.public_functions += module.public_api_summary.public_functions;
    collect_import_signals(&module.import_summary, signals, dependency_lookup);
}

fn collect_import_signals(
    import_summary: &RustReasoningImportFacts,
    signals: &mut ProfileSignals,
    dependency_lookup: &DependencySignalLookup<'_>,
) {
    for segments in &import_summary.production_external_imports {
        add_import_signal(segments, signals, dependency_lookup);
    }
}

fn add_import_signal(
    segments: &[String],
    signals: &mut ProfileSignals,
    dependency_lookup: &DependencySignalLookup<'_>,
) {
    if segments.is_empty() {
        return;
    }
    let label = compact_import_label(segments);
    let mut responsibilities = standard_import_responsibilities(segments);
    if let Some(dependency) = dependency_lookup.matching_cargo_dependency(segments) {
        let dependency_label = compact_dependency_label(dependency);
        signals.dependency_roots.insert(dependency_label.clone());
        let configured_responsibilities =
            dependency_lookup.configured_dependency_responsibilities(dependency);
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

fn recommended_hint_path(project_root: &Path, package_root: &Path, owner_path: &Path) -> PathBuf {
    owner_path
        .strip_prefix(project_root)
        .or_else(|_| owner_path.strip_prefix(package_root))
        .map_or_else(|_| owner_path.to_path_buf(), Path::to_path_buf)
}
