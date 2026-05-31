//! RFC `search prime` renderer for bounded package and workspace maps.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::discovery::{discover_cargo_package_roots, rust_project_harness_scope};
use crate::parser::{
    RustReasoningOwnerBranchFacts, RustReasoningOwnerBranchRole, RustReasoningTreeFacts,
    parse_cargo_dependency_facts, parse_cargo_manifest, rust_reasoning_tree_facts,
};
use crate::rules::evaluate_default_rule_packs_with_config;
use crate::{RustHarnessConfig, RustHarnessFinding};

use super::cargo::manifest_features;
use super::context::parse_scope;
use super::format::{
    display_project_path, package_label, render_cargo_dependency_line, render_owner_line,
    resolve_package_root, should_run_member_scopes,
};
use super::limits::{
    PRIME_EDGE_LIMIT, PRIME_FINDING_LIMIT, PRIME_OWNER_LIMIT, WORKSPACE_INDEX_PACKAGE_LIMIT,
    WORKSPACE_INDEX_THRESHOLD,
};
use super::prime_support::{
    api_candidate_lines, cfg_lines, child_edge_count, child_edge_lines, dependency_labels,
    feature_lines, grouped_finding_lines, owner_dependency_lines, surface_line, target_labels,
    target_lines, test_surface_line,
};
use super::recency::compare_paths_by_recency;

/// Render the RFC search-prime packet for a project root.
///
/// # Errors
///
/// Returns an error when the project root does not exist or when a selected
/// package cannot be resolved.
pub fn render_rust_project_harness_search_prime(project_root: &Path) -> Result<String, String> {
    render_rust_project_harness_search_prime_with_config(
        project_root,
        &RustHarnessConfig::default(),
        None,
    )
}

/// Render the RFC search-prime packet with explicit harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist or when a selected
/// package cannot be resolved.
pub fn render_rust_project_harness_search_prime_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
    selected_package: Option<&str>,
) -> Result<String, String> {
    render_prime_with_config(
        project_root,
        config,
        selected_package,
        PrimeRenderMode::Full,
    )
}

pub(super) fn render_search_prime(
    project_root: &Path,
    config: &RustHarnessConfig,
    selected_package: Option<&str>,
    seed_limit: Option<usize>,
) -> Result<String, String> {
    render_prime_with_config(
        project_root,
        config,
        selected_package,
        prime_render_mode(seed_limit),
    )
}

fn render_prime_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
    selected_package: Option<&str>,
    mode: PrimeRenderMode,
) -> Result<String, String> {
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
    if let Some(selected_package) = selected_package {
        let package_root = resolve_package_root(project_root, &package_roots, selected_package)?;
        return Ok(render_package_prime(
            project_root,
            &package_root,
            config,
            mode,
        ));
    }
    if package_roots.len() > WORKSPACE_INDEX_THRESHOLD {
        return Ok(render_workspace_index_prime(project_root, &package_roots));
    }
    if package_roots.len() == 1 {
        return Ok(render_package_prime(
            project_root,
            &package_roots[0],
            config,
            mode,
        ));
    }

    let mut rendered = String::new();
    for package_root in package_roots {
        let package_prime = render_package_prime(project_root, &package_root, config, mode);
        if !rendered.is_empty() && !rendered.ends_with('\n') && !package_prime.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&package_prime);
    }
    Ok(rendered)
}

#[derive(Clone, Copy)]
enum PrimeRenderMode {
    Full,
    SeedSource { seed_limit: usize },
}

fn prime_render_mode(seed_limit: Option<usize>) -> PrimeRenderMode {
    if let Some(seed_limit) = seed_limit {
        PrimeRenderMode::SeedSource { seed_limit }
    } else {
        PrimeRenderMode::Full
    }
}

fn render_workspace_index_prime(project_root: &Path, package_roots: &[PathBuf]) -> String {
    let mut rendered = format!(
        "[search-prime] mode=workspace-index workspace=large packages={}\n",
        package_roots.len()
    );
    for package_root in package_roots.iter().take(WORKSPACE_INDEX_PACKAGE_LIMIT) {
        let label = package_label(project_root, package_root);
        let _ = writeln!(
            rendered,
            "|package {label} next=package:{}",
            display_project_path(project_root, package_root)
        );
    }
    if package_roots.len() > WORKSPACE_INDEX_PACKAGE_LIMIT {
        let _ = writeln!(
            rendered,
            "|note truncated_packages={}",
            package_roots.len() - WORKSPACE_INDEX_PACKAGE_LIMIT
        );
    }
    rendered
}

fn render_package_prime(
    project_root: &Path,
    package_root: &Path,
    config: &RustHarnessConfig,
    mode: PrimeRenderMode,
) -> String {
    if let PrimeRenderMode::SeedSource { seed_limit } = mode {
        return render_package_prime_seed_source(project_root, package_root, config, seed_limit);
    }
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let parsed_modules = parse_scope(&scope, config);
    let findings = match mode {
        PrimeRenderMode::Full => {
            evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config)
        }
        PrimeRenderMode::SeedSource { .. } => Vec::new(),
    };
    let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
    let source_modules = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module)
        .count();
    let owner_dependencies = reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .collect::<Vec<_>>();
    let owner_branches = ranked_owner_branches(package_root, &reasoning_tree, &findings);
    let cargo_dependencies = parse_cargo_dependency_facts(package_root);
    let cargo_manifest = parse_cargo_manifest(package_root);
    let features = manifest_features(package_root);
    let package_label = package_label(project_root, package_root);
    let module_by_path = parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let mut rendered = format!(
        "[search-prime] mode=package package={package_label} src={source_modules} own={} edge={} find={} dep={}\n",
        reasoning_tree.owner_branches.len(),
        owner_dependencies.len() + child_edge_count(&reasoning_tree),
        findings.len(),
        cargo_dependencies.len()
    );
    let _ = writeln!(
        rendered,
        "|package {package_label} t={} dep={}",
        target_labels(&scope),
        dependency_labels(&cargo_dependencies)
    );
    for dependency in cargo_dependencies.iter().take(6) {
        let _ = writeln!(rendered, "{}", render_cargo_dependency_line(dependency));
    }
    for line in feature_lines(&features) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in cfg_lines(package_root, &parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in target_lines(package_root, &cargo_manifest) {
        let _ = writeln!(rendered, "{line}");
    }
    if let Some(line) = surface_line(&parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    if let Some(line) = test_surface_line(&scope) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in api_candidate_lines(package_root, &parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    for branch in owner_branches.iter().take(PRIME_OWNER_LIMIT) {
        let _ = writeln!(
            rendered,
            "{}",
            render_owner_line(
                package_root,
                branch,
                module_by_path.get(&branch.path).copied()
            )
        );
    }
    for edge in child_edge_lines(package_root, &reasoning_tree)
        .into_iter()
        .chain(owner_dependency_lines(package_root, &owner_dependencies))
        .take(PRIME_EDGE_LIMIT)
    {
        let _ = writeln!(rendered, "{edge}");
    }
    for finding_line in grouped_finding_lines(package_root, &findings)
        .into_iter()
        .take(PRIME_FINDING_LIMIT)
    {
        let _ = writeln!(rendered, "{finding_line}");
    }
    let next = owner_branches
        .iter()
        .take(3)
        .map(|branch| format!("owner:{}", display_project_path(package_root, &branch.path)))
        .collect::<Vec<_>>();
    if !next.is_empty() {
        let _ = writeln!(rendered, "|next {}", next.join(","));
    }
    rendered
}

fn render_package_prime_seed_source(
    project_root: &Path,
    package_root: &Path,
    config: &RustHarnessConfig,
    seed_limit: usize,
) -> String {
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let mut owner_paths =
        crate::discovery::discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names);
    owner_paths.sort_by(|left, right| {
        owner_rank_for_path(package_root, right)
            .cmp(&owner_rank_for_path(package_root, left))
            .then_with(|| compare_paths_by_recency(package_root, left, right))
    });
    let cargo_dependencies = parse_cargo_dependency_facts(package_root);
    let features = manifest_features(package_root);
    let package_label = package_label(project_root, package_root);
    let mut rendered = String::new();
    rendered.push_str("[search-prime] ");
    let _ = write!(rendered, "mode=package package={package_label} ");
    let _ = write!(rendered, "src={} ", owner_paths.len());
    let _ = write!(rendered, "own={} ", owner_paths.len());
    let _ = write!(rendered, "edge=0 ");
    let _ = write!(rendered, "dep={}", cargo_dependencies.len());
    rendered.push('\n');
    let feature_names = feature_seed_names(&features, seed_limit.max(1));
    if !feature_names.is_empty() {
        let _ = writeln!(rendered, "|seed features:{}", feature_names.join(","));
    }
    let cfg_names = feature_names
        .iter()
        .map(|name| format!("feature:{name}"))
        .collect::<Vec<_>>();
    if !cfg_names.is_empty() {
        let _ = writeln!(rendered, "|seed cfg:{}", cfg_names.join(","));
    }
    let owner_limit = seed_limit.min(owner_paths.len());
    let owners = owner_paths
        .iter()
        .take(owner_limit)
        .map(|path| display_project_path(project_root, path))
        .collect::<Vec<_>>();
    if !owners.is_empty() {
        let _ = writeln!(rendered, "|seed owner:{}", owners.join(","));
    }
    let docs = docs_seed_names_from_paths(&owner_paths, seed_limit.max(1));
    if !docs.is_empty() {
        let _ = writeln!(rendered, "|seed docs:{}", docs.join(","));
    }
    if owner_paths.iter().any(|path| {
        let displayed = display_project_path(package_root, path);
        displayed.starts_with("tests/")
            || displayed.starts_with("benches/")
            || displayed.starts_with("examples/")
    }) {
        let _ = writeln!(rendered, "|seed tests");
    }
    if owner_paths.len() > owner_limit {
        let _ = writeln!(
            rendered,
            "|note seeds_truncated={} limit={}",
            owner_paths.len() - owner_limit,
            seed_limit
        );
    }
    rendered
}

fn feature_seed_names(features: &[(String, Vec<String>)], seed_limit: usize) -> Vec<&str> {
    let mut names = features
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    names.sort_by_key(|name| match *name {
        "default" => 90,
        "full" => 80,
        _ => 0,
    });
    names.truncate(seed_limit);
    names
}

fn docs_seed_names_from_paths(paths: &[PathBuf], seed_limit: usize) -> Vec<String> {
    let mut type_names = Vec::new();
    let mut module_names = Vec::new();
    let mut function_names = Vec::new();
    let mut other_names = Vec::new();
    for path in paths {
        let displayed = path.to_string_lossy().replace('\\', "/");
        if displayed.contains("/tests/")
            || displayed.contains("/benches/")
            || displayed.contains("/examples/")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        collect_docs_seed_names(
            &text,
            &mut type_names,
            &mut module_names,
            &mut function_names,
            &mut other_names,
        );
    }
    function_names.sort_by(|left, right| right.cmp(left));
    let mut names = Vec::new();
    append_unique_names(&mut names, type_names, seed_limit);
    append_unique_names(&mut names, module_names, seed_limit);
    append_unique_names(&mut names, function_names, seed_limit);
    append_unique_names(&mut names, other_names, seed_limit);
    names.truncate(seed_limit);
    names
}

fn collect_docs_seed_names(
    text: &str,
    type_names: &mut Vec<String>,
    module_names: &mut Vec<String>,
    function_names: &mut Vec<String>,
    other_names: &mut Vec<String>,
) {
    for line in text.lines() {
        let mut tokens = line
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .filter(|token| !token.is_empty());
        while let Some(token) = tokens.next() {
            if !matches!(
                token,
                "fn" | "struct" | "enum" | "trait" | "type" | "const" | "static" | "mod"
            ) {
                continue;
            };
            let Some(name) = tokens.next() else {
                continue;
            };
            match token {
                "struct" | "enum" | "trait" | "type" => type_names.push(name.to_string()),
                "mod" => module_names.push(name.to_string()),
                "fn" => function_names.push(name.to_string()),
                _ => other_names.push(name.to_string()),
            }
        }
    }
}

fn append_unique_names(names: &mut Vec<String>, candidates: Vec<String>, seed_limit: usize) {
    for candidate in candidates {
        if names.len() >= seed_limit {
            return;
        }
        if !names.iter().any(|existing| existing == &candidate) {
            names.push(candidate);
        }
    }
}

fn owner_rank_for_path(package_root: &Path, path: &Path) -> usize {
    let displayed = display_project_path(package_root, path);
    match displayed.as_str() {
        _ if displayed.ends_with("/mod.rs") => 110,
        "src/lib.rs" | "src/main.rs" => 100,
        _ if displayed.starts_with("src/") => 60,
        _ if displayed.starts_with("tests/") => 30,
        _ => 10,
    }
}

fn ranked_owner_branches<'a>(
    package_root: &Path,
    reasoning_tree: &'a RustReasoningTreeFacts,
    findings: &[RustHarnessFinding],
) -> Vec<&'a RustReasoningOwnerBranchFacts> {
    let finding_paths = findings
        .iter()
        .filter_map(|finding| finding.location.path.as_ref())
        .collect::<Vec<_>>();
    let mut branches = reasoning_tree.owner_branches.iter().collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        owner_rank(right, &finding_paths)
            .cmp(&owner_rank(left, &finding_paths))
            .then_with(|| compare_paths_by_recency(package_root, &left.path, &right.path))
    });
    branches
}

fn owner_rank(branch: &RustReasoningOwnerBranchFacts, finding_paths: &[&PathBuf]) -> usize {
    let mut rank = 0;
    if branch.roles.contains(&RustReasoningOwnerBranchRole::Root) {
        rank += 100;
    }
    if branch.roles.contains(&RustReasoningOwnerBranchRole::Facade) {
        rank += 80;
    }
    if branch
        .roles
        .contains(&RustReasoningOwnerBranchRole::Interface)
    {
        rank += 70;
    }
    if finding_paths.contains(&&branch.path) {
        rank += 60;
    }
    if branch.import_summary.external_imports > 0 {
        rank += 30;
    }
    if !branch.declared_child_edges.is_empty() {
        rank += 20;
    }
    rank
}
