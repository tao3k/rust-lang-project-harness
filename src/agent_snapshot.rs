//! Low-noise agent snapshot rendering from parser reasoning-tree facts.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::model::RustHarnessConfig;
use crate::parser::{
    RustModuleChildEdge, RustReasoningImportFacts, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustReasoningOwnerDependencyFacts, parse_rust_file,
    rust_reasoning_tree_facts,
};
use crate::rules::evaluate_default_rule_packs_with_config;
use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustProjectHarnessScope};

/// Render a compact project-structure snapshot for repair-oriented agents.
///
/// The snapshot is derived from parser reasoning-tree facts and intentionally
/// avoids the full `RustHarnessReport` JSON shape.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn render_rust_project_harness_agent_snapshot(project_root: &Path) -> Result<String, String> {
    render_rust_project_harness_agent_snapshot_with_config(
        project_root,
        &RustHarnessConfig::default(),
    )
}

/// Render a compact project-structure snapshot with explicit harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn render_rust_project_harness_agent_snapshot_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
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
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "[agent:snapshot] {} rust",
        display_path(project_root)
    );
    let _ = writeln!(rendered, "Packages: {}", package_roots.len());
    for package_root in package_roots {
        let scope = rust_project_harness_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let parsed_modules = parse_scope(&scope, config);
        let findings =
            evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config);
        rendered.push('\n');
        rendered.push_str(&render_package_snapshot(&scope, &parsed_modules, &findings));
    }
    Ok(rendered)
}

fn render_package_snapshot(
    scope: &RustProjectHarnessScope,
    parsed_modules: &[crate::parser::ParsedRustModule],
    findings: &[RustHarnessFinding],
) -> String {
    let reasoning_tree = rust_reasoning_tree_facts(scope, parsed_modules);
    let source_module_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module)
        .count();
    let root_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_module_tree_root)
        .count();
    let branch_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module && !module.declared_child_edges.is_empty())
        .count();
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Package: {}",
        display_project_path(&reasoning_tree.package_root, &reasoning_tree.package_root)
    );
    let _ = writeln!(
        rendered,
        "SourceRoots: {}",
        display_paths(
            &reasoning_tree.package_root,
            &reasoning_tree.source_roots,
            "-"
        )
    );
    let _ = writeln!(
        rendered,
        "PackageEntrypoints: {}",
        display_paths(
            &reasoning_tree.package_root,
            &reasoning_tree.package_entrypoints,
            "-"
        )
    );
    let _ = writeln!(
        rendered,
        "Modules: source={} roots={} branches={} deps={} shadowed={} orphaned={}",
        source_module_count,
        root_count,
        branch_count,
        reasoning_tree.owner_dependencies.len(),
        reasoning_tree.shadowed_module_sources.len(),
        reasoning_tree.unreachable_source_files.len()
    );
    rendered.push_str("OwnerBranches:\n");
    let branch_lines = reasoning_tree
        .owner_branches
        .iter()
        .map(|branch| {
            format!(
                " - {} [{}] owner={} -> {}",
                display_project_path(&reasoning_tree.package_root, &branch.path),
                owner_branch_role_labels(branch).join(", "),
                display_owner_namespace(branch) + &display_import_summary(&branch.import_summary),
                display_child_edges(
                    &reasoning_tree.package_root,
                    &branch.declared_child_edges,
                    "-"
                )
            )
        })
        .collect::<Vec<_>>();
    if branch_lines.is_empty() {
        rendered.push_str(" - none\n");
    } else {
        rendered.push_str(&branch_lines.join("\n"));
        rendered.push('\n');
    }
    rendered.push_str("OwnerDependencies:\n");
    let dependency_lines = reasoning_tree
        .owner_dependencies
        .iter()
        .map(|dependency| display_owner_dependency(&reasoning_tree.package_root, dependency))
        .collect::<Vec<_>>();
    if dependency_lines.is_empty() {
        rendered.push_str(" - none\n");
    } else {
        rendered.push_str(&dependency_lines.join("\n"));
        rendered.push('\n');
    }
    rendered.push_str("FindingGroups:\n");
    let finding_lines = grouped_findings(&reasoning_tree.package_root, findings);
    if finding_lines.is_empty() {
        rendered.push_str(" - none\n");
    } else {
        rendered.push_str(&finding_lines.join("\n"));
        rendered.push('\n');
    }
    rendered
}

fn parse_scope(
    scope: &RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<crate::parser::ParsedRustModule> {
    discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn owner_branch_role_labels(branch: &RustReasoningOwnerBranchFacts) -> Vec<String> {
    branch
        .roles
        .iter()
        .map(|role| match role {
            RustReasoningOwnerBranchRole::Root => "root".to_string(),
            RustReasoningOwnerBranchRole::Facade => "facade".to_string(),
            RustReasoningOwnerBranchRole::Interface => "interface".to_string(),
            RustReasoningOwnerBranchRole::Binary => "binary".to_string(),
            RustReasoningOwnerBranchRole::PackageEntrypoint => "package-entrypoint".to_string(),
            RustReasoningOwnerBranchRole::RepeatedNamespace(segments) => {
                format!("repeated:{}", segments.join(","))
            }
            RustReasoningOwnerBranchRole::Branch => "branch".to_string(),
        })
        .collect()
}

fn display_owner_namespace(branch: &RustReasoningOwnerBranchFacts) -> String {
    if branch.owner_namespace.is_empty() {
        return "-".to_string();
    }
    branch.owner_namespace.join("/")
}

fn display_import_summary(summary: &RustReasoningImportFacts) -> String {
    if summary.total_imports == 0 {
        return String::new();
    }
    let mut parts = Vec::new();
    push_count(&mut parts, "crate", summary.crate_imports);
    push_count(&mut parts, "self", summary.self_imports);
    push_count(&mut parts, "parent", summary.parent_imports);
    push_count(&mut parts, "external", summary.external_imports);
    push_count(&mut parts, "absolute", summary.absolute_imports);
    push_count(&mut parts, "unknown", summary.unknown_imports);
    push_count(&mut parts, "glob", summary.glob_imports);
    push_count(&mut parts, "deep", summary.deep_relative_imports);
    push_count(&mut parts, "prelude", summary.prelude_imports);
    push_count(&mut parts, "test", summary.test_context_imports);
    format!(" imports={}", parts.join(","))
}

fn push_count(parts: &mut Vec<String>, label: &str, count: usize) {
    if count > 0 {
        parts.push(format!("{label}:{count}"));
    }
}

fn display_owner_dependency(
    package_root: &Path,
    dependency: &RustReasoningOwnerDependencyFacts,
) -> String {
    format!(
        " - {} --{}--> {}",
        display_project_path(package_root, &dependency.source_path),
        import_root_label(dependency.via_root),
        display_project_path(package_root, &dependency.target_path)
    )
}

fn import_root_label(root: crate::parser::RustUseImportRootKind) -> &'static str {
    match root {
        crate::parser::RustUseImportRootKind::Absolute => "absolute",
        crate::parser::RustUseImportRootKind::Crate => "crate",
        crate::parser::RustUseImportRootKind::SelfScope => "self",
        crate::parser::RustUseImportRootKind::Parent => "parent",
        crate::parser::RustUseImportRootKind::External => "external",
        crate::parser::RustUseImportRootKind::Unknown => "unknown",
    }
}

fn grouped_findings(package_root: &Path, findings: &[RustHarnessFinding]) -> Vec<String> {
    let mut groups = BTreeMap::<(RustDiagnosticSeverity, String, String), FindingGroup>::new();
    for finding in findings {
        let key = (
            finding.severity,
            finding.rule_id.clone(),
            finding.title.clone(),
        );
        let group = groups.entry(key).or_insert_with(|| FindingGroup {
            count: 0,
            first_path: finding.location.path.clone(),
        });
        group.count += 1;
    }
    groups
        .into_iter()
        .map(|((severity, rule_id, title), group)| {
            let first_path = group.first_path.as_ref().map_or_else(
                || "<memory>".to_string(),
                |path| display_project_path(package_root, path),
            );
            format!(
                " - {} {} x{} first={} {}",
                severity.as_str(),
                rule_id,
                group.count,
                first_path,
                title
            )
        })
        .collect()
}

#[derive(Debug)]
struct FindingGroup {
    count: usize,
    first_path: Option<PathBuf>,
}

fn display_paths(package_root: &Path, paths: &[PathBuf], empty: &str) -> String {
    if paths.is_empty() {
        return empty.to_string();
    }
    paths
        .iter()
        .map(|path| display_project_path(package_root, path))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_child_edges(package_root: &Path, edges: &[RustModuleChildEdge], empty: &str) -> String {
    if edges.is_empty() {
        return empty.to_string();
    }
    edges
        .iter()
        .map(|edge| {
            format!(
                "{}:{}",
                edge.kind.as_str(),
                display_project_path(package_root, &edge.child_path)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_project_path(package_root: &Path, path: &Path) -> String {
    path.strip_prefix(package_root)
        .map_or_else(|_| display_path(path), display_path)
}

fn display_path(path: &Path) -> String {
    let rendered = path.display().to_string().replace('\\', "/");
    if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    }
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}
