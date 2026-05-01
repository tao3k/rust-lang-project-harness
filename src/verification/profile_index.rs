//! Parser-native responsibility profile suggestions for verification config.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::RustProjectHarnessScope;
use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::model::RustHarnessConfig;
use crate::parser::{
    ParsedRustModule, RustReasoningModuleFacts, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustUseImportRootKind, parse_rust_file,
    rust_reasoning_tree_facts,
};

use super::profile::{responsibility_labels, task_kind_labels, task_kinds_for_responsibilities};
use super::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationTaskKind,
};

/// Whether a parser-suggested verification profile is already configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustVerificationProfileCandidateState {
    /// No matching profile hint exists for this parser owner.
    MissingProfile,
    /// A profile hint exists, but parser facts suggest additional responsibilities.
    ProfileDrift,
    /// A profile hint covers all parser-suggested responsibilities.
    Configured,
}

impl RustVerificationProfileCandidateState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::MissingProfile => "missing_profile",
            Self::ProfileDrift => "profile_drift",
            Self::Configured => "configured",
        }
    }

    const fn requires_action(self) -> bool {
        matches!(self, Self::MissingProfile | Self::ProfileDrift)
    }
}

/// Searchable responsibility-profile candidates derived from parser facts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationProfileIndex {
    /// Root used to compact owner paths in agent renders.
    #[serde(default, skip_serializing_if = "path_buf_is_empty")]
    pub project_root: PathBuf,
    /// Parser-suggested owner profile candidates.
    pub candidates: Vec<RustVerificationProfileCandidate>,
}

impl RustVerificationProfileIndex {
    /// Return whether no owner still needs profile configuration.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.active_candidates().is_empty()
    }

    /// Return candidates that still need agent action.
    #[must_use]
    pub fn active_candidates(&self) -> Vec<&RustVerificationProfileCandidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.requires_action())
            .collect()
    }

    /// Return candidates owned by one package root.
    #[must_use]
    pub fn candidates_for_package(
        &self,
        package_root: impl AsRef<Path>,
    ) -> Vec<&RustVerificationProfileCandidate> {
        let package_root = package_root.as_ref();
        self.candidates
            .iter()
            .filter(|candidate| {
                candidate.package_root == package_root
                    || candidate
                        .package_root
                        .strip_prefix(&self.project_root)
                        .is_ok_and(|relative| relative == package_root)
            })
            .collect()
    }

    /// Return suggested profile hints for candidates that still need action.
    #[must_use]
    pub fn active_profile_hints(&self) -> Vec<RustVerificationProfileHint> {
        self.active_candidates()
            .into_iter()
            .map(RustVerificationProfileCandidate::to_profile_hint)
            .collect()
    }
}

/// One owner profile candidate an Agent can turn into config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationProfileCandidate {
    /// Cargo package root that owns the parser facts.
    pub package_root: PathBuf,
    /// Owner module path.
    pub owner_path: PathBuf,
    /// Recommended path to use in `RustVerificationProfileHint`.
    pub hint_path: PathBuf,
    /// Parser-derived owner namespace.
    pub owner_namespace: Vec<String>,
    /// Whether the current config covers this candidate.
    pub state: RustVerificationProfileCandidateState,
    /// Responsibilities suggested by parser facts.
    pub suggested_responsibilities: BTreeSet<RustOwnerResponsibility>,
    /// Responsibilities already configured by a matching profile hint.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub configured_responsibilities: BTreeSet<RustOwnerResponsibility>,
    /// Effective task kinds implied by the suggested responsibilities and policy mapping.
    pub suggested_task_kinds: BTreeSet<RustVerificationTaskKind>,
    /// Compact parser facts behind the suggestion.
    pub evidence: Vec<RustVerificationEvidence>,
}

impl RustVerificationProfileCandidate {
    /// Return whether this candidate still needs agent action.
    #[must_use]
    pub const fn requires_action(&self) -> bool {
        self.state.requires_action()
    }

    /// Convert this candidate into a profile hint using the recommended path.
    #[must_use]
    pub fn to_profile_hint(&self) -> RustVerificationProfileHint {
        RustVerificationProfileHint::new(
            self.hint_path.clone(),
            self.suggested_responsibilities.iter().copied(),
        )
    }
}

#[derive(Debug, Default)]
struct ProfileSignals {
    public_items: usize,
    public_exports: usize,
    public_functions: usize,
    owner_deps: usize,
    child_modules: usize,
    network_roots: BTreeSet<String>,
    persistence_roots: BTreeSet<String>,
    security_roots: BTreeSet<String>,
    performance_roots: BTreeSet<String>,
    path_signals: BTreeSet<String>,
}

/// Build parser-suggested responsibility profile candidates for a Rust project.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index(
    project_root: &Path,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_config(project_root, &RustHarnessConfig::default())
}

/// Build parser-suggested responsibility profile candidates with config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_policy(
        project_root,
        config,
        &config.verification_policy,
    )
}

/// Build parser-suggested responsibility profile candidates with policy.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_policy(
    project_root: &Path,
    config: &RustHarnessConfig,
    policy: &RustVerificationPolicy,
) -> Result<RustVerificationProfileIndex, String> {
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
    let mut candidates = Vec::new();
    for package_root in package_roots {
        let scope = rust_project_harness_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let parsed_modules = parse_scope(&scope, config);
        let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
        collect_package_candidates(
            project_root,
            &reasoning_tree.package_root,
            &reasoning_tree.modules,
            &reasoning_tree.owner_branches,
            &parsed_modules,
            policy,
            &mut candidates,
        );
    }
    candidates.sort_by(|left, right| {
        left.package_root
            .cmp(&right.package_root)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
    });
    Ok(RustVerificationProfileIndex {
        project_root: project_root.to_path_buf(),
        candidates,
    })
}

/// Render active responsibility-profile candidates for agents.
#[must_use]
pub fn render_rust_verification_profile_index(index: &RustVerificationProfileIndex) -> String {
    let display_root = if index.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(index.project_root.as_path())
    };
    index
        .active_candidates()
        .into_iter()
        .map(|candidate| render_profile_candidate(candidate, display_root))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render responsibility-profile candidates as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the index cannot be encoded as JSON.
pub fn render_rust_verification_profile_index_json(
    index: &RustVerificationProfileIndex,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(index)
}

fn collect_package_candidates(
    project_root: &Path,
    package_root: &Path,
    modules: &[RustReasoningModuleFacts],
    branches: &[RustReasoningOwnerBranchFacts],
    parsed_modules: &[ParsedRustModule],
    policy: &RustVerificationPolicy,
    candidates: &mut Vec<RustVerificationProfileCandidate>,
) {
    let parsed_by_path = parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let branches_by_path = branches
        .iter()
        .map(|branch| (branch.path.clone(), branch))
        .collect::<BTreeMap<_, _>>();
    for module in modules.iter().filter(|module| module.is_source_module) {
        let Some(parsed_module) = parsed_by_path.get(&module.path) else {
            continue;
        };
        let branch = branches_by_path.get(&module.path).copied();
        let signals = profile_signals(module, branch, parsed_module);
        let responsibilities = suggested_responsibilities(module, branch, &signals);
        if responsibilities.is_empty() {
            continue;
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
}

fn profile_signals(
    module: &RustReasoningModuleFacts,
    branch: Option<&RustReasoningOwnerBranchFacts>,
    parsed_module: &ParsedRustModule,
) -> ProfileSignals {
    let mut signals = ProfileSignals {
        owner_deps: module
            .import_summary
            .local_owner_dependencies
            .iter()
            .filter(|dependency| !dependency.is_test_context)
            .count(),
        child_modules: branch.map_or(0, |branch| branch.declared_child_edges.len()),
        ..ProfileSignals::default()
    };
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
    collect_import_signals(parsed_module, &mut signals);
    collect_path_signals(&module.source_path.namespace_components, &mut signals);
    signals
}

fn collect_import_signals(parsed_module: &ParsedRustModule, signals: &mut ProfileSignals) {
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
            add_import_signal(&import.segments, signals);
        }
    }
}

fn add_import_signal(segments: &[String], signals: &mut ProfileSignals) {
    let Some(root) = segments.first().map(String::as_str) else {
        return;
    };
    let label = compact_import_label(segments);
    if import_is_network_boundary(root, segments) {
        signals.network_roots.insert(label.clone());
    }
    if import_is_persistence_boundary(root, segments) {
        signals.persistence_roots.insert(label.clone());
    }
    if import_is_security_boundary(root, segments) {
        signals.security_roots.insert(label.clone());
    }
    if import_is_performance_boundary(root, segments) {
        signals.performance_roots.insert(label);
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

fn collect_path_signals(namespace: &[String], signals: &mut ProfileSignals) {
    for segment in namespace.iter().map(|segment| segment.to_ascii_lowercase()) {
        if path_segment_has_any(
            &segment,
            &[
                "gateway",
                "router",
                "server",
                "webhook",
                "transport",
                "client",
                "runtime",
                "jobs",
            ],
        ) {
            signals.path_signals.insert(segment.clone());
        }
        if path_segment_has_any(
            &segment,
            &[
                "storage",
                "store",
                "cache",
                "checkpoint",
                "repo",
                "index",
                "db",
                "duckdb",
                "valkey",
                "redis",
                "lance",
                "parquet",
            ],
        ) {
            signals.path_signals.insert(segment.clone());
        }
        if path_segment_has_any(
            &segment,
            &[
                "auth",
                "acl",
                "token",
                "secret",
                "credential",
                "permission",
                "trust",
                "sandbox",
                "security",
                "policy",
            ],
        ) {
            signals.path_signals.insert(segment.clone());
        }
        if path_segment_has_any(
            &segment,
            &[
                "perf",
                "performance",
                "benchmark",
                "search",
                "tokenizer",
                "window",
            ],
        ) {
            signals.path_signals.insert(segment);
        }
    }
}

fn suggested_responsibilities(
    module: &RustReasoningModuleFacts,
    branch: Option<&RustReasoningOwnerBranchFacts>,
    signals: &ProfileSignals,
) -> BTreeSet<RustOwnerResponsibility> {
    let mut responsibilities = BTreeSet::new();
    if signals.public_functions > 0
        || signals.public_items > 0
        || signals.public_exports > 0
        || branch.is_some_and(branch_is_public_surface)
    {
        responsibilities.insert(RustOwnerResponsibility::PublicApi);
    }
    if !signals.network_roots.is_empty()
        || !signals.persistence_roots.is_empty()
        || signals.owner_deps >= 3
    {
        responsibilities.insert(RustOwnerResponsibility::ExternalDependency);
    }
    if !signals.persistence_roots.is_empty()
        || namespace_has_any(
            &module.source_path.namespace_components,
            &[
                "storage",
                "store",
                "cache",
                "checkpoint",
                "repo",
                "index",
                "db",
            ],
        )
    {
        responsibilities.insert(RustOwnerResponsibility::Persistence);
    }
    if !signals.security_roots.is_empty()
        || namespace_has_any(
            &module.source_path.namespace_components,
            &[
                "auth",
                "acl",
                "token",
                "secret",
                "credential",
                "permission",
                "trust",
                "sandbox",
                "security",
                "policy",
            ],
        )
    {
        responsibilities.insert(RustOwnerResponsibility::SecurityBoundary);
    }
    if !signals.performance_roots.is_empty()
        || namespace_has_any(
            &module.source_path.namespace_components,
            &[
                "perf",
                "performance",
                "benchmark",
                "search",
                "tokenizer",
                "window",
            ],
        )
    {
        responsibilities.insert(RustOwnerResponsibility::LatencySensitive);
    }
    if !signals.network_roots.is_empty()
        || namespace_has_any(
            &module.source_path.namespace_components,
            &[
                "gateway",
                "server",
                "webhook",
                "transport",
                "runtime",
                "jobs",
            ],
        )
    {
        responsibilities.insert(RustOwnerResponsibility::AvailabilityCritical);
    }
    responsibilities
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

fn import_is_network_boundary(root: &str, segments: &[String]) -> bool {
    matches!(
        root,
        "axum" | "tonic" | "tower" | "hyper" | "http" | "reqwest" | "url"
    ) || matches!(segments, [first, second, ..] if first == "tokio" && second == "net")
}

fn import_is_persistence_boundary(root: &str, segments: &[String]) -> bool {
    matches!(
        root,
        "redis" | "duckdb" | "lance" | "parquet" | "datafusion" | "gix" | "walkdir" | "ignore"
    ) || matches!(segments, [first, second, ..] if (first == "std" || first == "tokio") && second == "fs")
        || segments
            .iter()
            .any(|segment| matches!(segment.as_str(), "RecordBatch" | "File" | "OpenOptions"))
}

fn import_is_security_boundary(root: &str, segments: &[String]) -> bool {
    matches!(
        root,
        "sha2" | "base64" | "jsonwebtoken" | "ring" | "rustls" | "secrecy"
    ) || segments.iter().any(|segment| {
        path_segment_has_any(
            &segment.to_ascii_lowercase(),
            &[
                "auth",
                "acl",
                "token",
                "secret",
                "credential",
                "permission",
                "trust",
                "sandbox",
            ],
        )
    })
}

fn import_is_performance_boundary(root: &str, segments: &[String]) -> bool {
    matches!(
        root,
        "rayon" | "hdrhistogram" | "criterion" | "divan" | "iai_callgrind"
    ) || segments.iter().any(|segment| {
        path_segment_has_any(
            &segment.to_ascii_lowercase(),
            &["histogram", "latency", "throughput", "benchmark"],
        )
    })
}

fn profile_evidence(signals: &ProfileSignals) -> Vec<RustVerificationEvidence> {
    let mut evidence = Vec::new();
    push_usize_evidence(&mut evidence, "public_items", signals.public_items);
    push_usize_evidence(&mut evidence, "public_exports", signals.public_exports);
    push_usize_evidence(&mut evidence, "public_fns", signals.public_functions);
    push_usize_evidence(&mut evidence, "owner_deps", signals.owner_deps);
    push_usize_evidence(&mut evidence, "child_modules", signals.child_modules);
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
    push_set_evidence(&mut evidence, "path_signals", &signals.path_signals);
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

fn render_profile_candidate(
    candidate: &RustVerificationProfileCandidate,
    display_root: Option<&Path>,
) -> String {
    let display_root = display_root.unwrap_or(&candidate.package_root);
    let mut rendered = format!(
        "[verify-profile] {}\n",
        display_project_path(display_root, &candidate.owner_path)
    );
    if !candidate.owner_namespace.is_empty() {
        let _ = writeln!(
            rendered,
            "   |owner: {}",
            candidate.owner_namespace.join("/")
        );
    }
    let _ = writeln!(rendered, "   |state: {}", candidate.state.as_str());
    if !candidate.configured_responsibilities.is_empty() {
        let _ = writeln!(
            rendered,
            "   |configured: {}",
            responsibility_labels(&candidate.configured_responsibilities)
        );
    }
    let _ = writeln!(
        rendered,
        "   |suggest: {}",
        responsibility_labels(&candidate.suggested_responsibilities)
    );
    let _ = writeln!(
        rendered,
        "   |tasks: {}",
        task_kind_labels(&candidate.suggested_task_kinds)
    );
    let _ = writeln!(
        rendered,
        "   |hint_path: {}",
        display_path(&candidate.hint_path)
    );
    for fact in &candidate.evidence {
        let _ = writeln!(rendered, "   |fact: {}={}", fact.label, fact.value);
    }
    rendered
}

fn display_project_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn namespace_has_any(namespace: &[String], needles: &[&str]) -> bool {
    namespace
        .iter()
        .any(|segment| path_segment_has_any(&segment.to_ascii_lowercase(), needles))
}

fn path_segment_has_any(segment: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| segment.contains(needle))
}

fn parse_scope(
    scope: &RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<ParsedRustModule> {
    discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}

fn path_buf_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}
