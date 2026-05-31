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
        return Ok(render_package_prime(project_root, &package_root, config));
    }
    if package_roots.len() > WORKSPACE_INDEX_THRESHOLD {
        return Ok(render_workspace_index_prime(project_root, &package_roots));
    }
    if package_roots.len() == 1 {
        return Ok(render_package_prime(
            project_root,
            &package_roots[0],
            config,
        ));
    }

    let mut rendered = String::new();
    for package_root in package_roots {
        let package_prime = render_package_prime(project_root, &package_root, config);
        if !rendered.is_empty() && !rendered.ends_with('\n') && !package_prime.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&package_prime);
    }
    Ok(rendered)
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
) -> String {
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let parsed_modules = parse_scope(&scope, config);
    let findings = evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config);
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
    let module_by_path = parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let cargo_dependencies = parse_cargo_dependency_facts(package_root);
    let cargo_manifest = parse_cargo_manifest(package_root);
    let features = manifest_features(package_root);
    let package_label = package_label(project_root, package_root);
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
