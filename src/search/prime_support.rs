use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::RustDiagnosticSeverity;
use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, RustModuleChildEdge, RustReasoningOwnerDependencyFacts,
    RustReasoningTreeFacts, RustUseImportRootKind,
};
use crate::{RustHarnessFinding, RustProjectHarnessScope};

use super::format::display_project_path;

const PRIME_FEATURE_LIMIT: usize = 8;
const PRIME_CFG_LIMIT: usize = 8;
const PRIME_API_CANDIDATE_LIMIT: usize = 8;

pub(super) fn child_edge_count(reasoning_tree: &RustReasoningTreeFacts) -> usize {
    reasoning_tree
        .owner_branches
        .iter()
        .map(|branch| branch.declared_child_edges.len())
        .sum()
}

pub(super) fn child_edge_lines(
    package_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
) -> Vec<String> {
    reasoning_tree
        .owner_branches
        .iter()
        .flat_map(|branch| {
            branch
                .declared_child_edges
                .iter()
                .map(move |edge| render_child_edge(package_root, &branch.path, edge))
        })
        .collect()
}

fn render_child_edge(
    package_root: &Path,
    source_path: &Path,
    edge: &RustModuleChildEdge,
) -> String {
    format!(
        "|edge O:{} -{}-> O:{}",
        display_project_path(package_root, source_path),
        edge.kind.as_str(),
        display_project_path(package_root, &edge.child_path)
    )
}

pub(super) fn owner_dependency_lines(
    package_root: &Path,
    dependencies: &[&RustReasoningOwnerDependencyFacts],
) -> Vec<String> {
    dependencies
        .iter()
        .map(|dependency| {
            format!(
                "|edge O:{} -crate:{}-> O:{}",
                display_project_path(package_root, &dependency.source_path),
                import_root_label(dependency.via_root),
                display_project_path(package_root, &dependency.target_path)
            )
        })
        .collect()
}

fn import_root_label(root: RustUseImportRootKind) -> &'static str {
    match root {
        RustUseImportRootKind::Absolute => "absolute",
        RustUseImportRootKind::Crate => "crate",
        RustUseImportRootKind::SelfScope => "self",
        RustUseImportRootKind::Parent => "parent",
        RustUseImportRootKind::External => "external",
        RustUseImportRootKind::Unknown => "unknown",
    }
}

pub(super) fn grouped_finding_lines(
    package_root: &Path,
    findings: &[RustHarnessFinding],
) -> Vec<String> {
    let mut groups = BTreeMap::<(RustDiagnosticSeverity, String), FindingGroup>::new();
    for finding in findings {
        let key = (finding.severity, finding.rule_id.clone());
        let group = groups.entry(key).or_insert_with(|| FindingGroup {
            count: 0,
            first_path: finding.location.path.clone(),
        });
        group.count += 1;
    }
    groups
        .into_iter()
        .map(|((severity, rule_id), group)| {
            let location = group
                .first_path
                .as_deref()
                .map(|path| format!("O:{}", display_project_path(package_root, path)))
                .unwrap_or_else(|| "memory".to_string());
            format!(
                "|find {rule_id} x{} at={} severity={}",
                group.count,
                location,
                severity.as_str()
            )
        })
        .collect()
}

#[derive(Debug)]
struct FindingGroup {
    count: usize,
    first_path: Option<PathBuf>,
}

pub(super) fn target_labels(scope: &RustProjectHarnessScope) -> String {
    let mut labels = Vec::new();
    if !scope.source_paths.is_empty() {
        labels.push("lib");
    }
    if !scope.test_paths.is_empty() {
        labels.push("test");
    }
    if scope
        .package_paths
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) == Some("build.rs"))
    {
        labels.push("build");
    }
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(",")
    }
}

pub(super) fn dependency_labels(dependencies: &[CargoDependencyFacts]) -> String {
    let labels = dependencies
        .iter()
        .filter(|dependency| dependency.target.is_none())
        .take(8)
        .map(|dependency| dependency.package_name.as_str())
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(",")
    }
}

pub(super) fn feature_lines(features: &[(String, Vec<String>)]) -> Vec<String> {
    features
        .iter()
        .take(PRIME_FEATURE_LIMIT)
        .map(|(name, enables)| {
            format!(
                "|feature {} dep={} next=features:{}",
                name,
                compact_feature_enables(enables),
                name
            )
        })
        .collect()
}

pub(super) fn cfg_lines(parsed_modules: &[ParsedRustModule]) -> Vec<String> {
    let mut cfgs = parsed_modules
        .iter()
        .flat_map(|module| {
            module
                .source
                .lines()
                .filter(|line| line.contains("cfg(") || line.contains("cfg_attr("))
                .filter_map(extract_cfg_label)
        })
        .collect::<Vec<_>>();
    cfgs.sort();
    cfgs.dedup();
    cfgs.into_iter()
        .take(PRIME_CFG_LIMIT)
        .map(|cfg| format!("|cfg {cfg} next=cfg:{cfg}"))
        .collect()
}

pub(super) fn surface_line(parsed_modules: &[ParsedRustModule]) -> Option<String> {
    let mut public_items = public_item_names(parsed_modules);
    public_items.truncate(8);
    (!public_items.is_empty()).then(|| {
        format!(
            "|surface public_api={} public_external=-",
            public_items.join(",")
        )
    })
}

pub(super) fn test_surface_line(scope: &RustProjectHarnessScope) -> Option<String> {
    (!scope.test_paths.is_empty()).then(|| {
        let tests = scope
            .test_paths
            .iter()
            .take(6)
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("tests")
            })
            .collect::<Vec<_>>();
        format!("|test-surface tests={} next=tests", tests.join(","))
    })
}

pub(super) fn api_candidate_lines(
    package_root: &Path,
    parsed_modules: &[ParsedRustModule],
) -> Vec<String> {
    parsed_modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .top_level_items
                .iter()
                .filter(|item| item.is_public)
                .filter_map(|item| {
                    item.name
                        .as_deref()
                        .or(item.function_name.as_deref())
                        .map(|name| {
                            format!(
                                "|api-candidate {} reason=public-item owner={} next=docs:{}",
                                name,
                                display_project_path(package_root, &module.report.path),
                                name
                            )
                        })
                })
        })
        .take(PRIME_API_CANDIDATE_LIMIT)
        .collect()
}

fn compact_feature_enables(enables: &[String]) -> String {
    if enables.is_empty() {
        "-".to_string()
    } else {
        enables
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn extract_cfg_label(line: &str) -> Option<String> {
    if let Some((_, tail)) = line.split_once("feature") {
        let quoted = tail.split('"').nth(1)?;
        return Some(format!("feature:{quoted}"));
    }
    let cfg = line
        .split("cfg")
        .nth(1)?
        .trim_start_matches("_attr")
        .trim_start_matches('(')
        .trim_start();
    let label = cfg
        .split(|character: char| character == ')' || character == ',' || character.is_whitespace())
        .next()
        .unwrap_or("")
        .trim_matches('"');
    (!label.is_empty()).then(|| label.to_string())
}

fn public_item_names(parsed_modules: &[ParsedRustModule]) -> Vec<String> {
    let mut names = parsed_modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .top_level_items
                .iter()
                .filter(|item| item.is_public)
                .filter_map(|item| item.name.clone().or_else(|| item.function_name.clone()))
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}
