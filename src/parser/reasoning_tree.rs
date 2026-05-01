//! Project reasoning-tree facts derived from parsed Rust modules.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::RustProjectHarnessScope;

use super::module_tree::{
    RustModuleChildEdge, RustModuleSourceShadow, external_child_module_edges, is_module_tree_root,
    rust_module_tree_facts,
};
use super::{ParsedRustModule, RustSourcePathFacts, rust_source_path_facts};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustReasoningTreeFacts {
    pub(crate) package_root: PathBuf,
    pub(crate) source_roots: Vec<PathBuf>,
    pub(crate) package_entrypoints: Vec<PathBuf>,
    pub(crate) modules: Vec<RustReasoningModuleFacts>,
    pub(crate) owner_branches: Vec<RustReasoningOwnerBranchFacts>,
    pub(crate) shadowed_module_sources: Vec<RustModuleSourceShadow>,
    pub(crate) unreachable_source_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningModuleFacts {
    pub(crate) path: PathBuf,
    pub(crate) source_path: RustSourcePathFacts,
    pub(crate) is_source_module: bool,
    pub(crate) is_module_tree_root: bool,
    pub(crate) declared_child_edges: Vec<RustModuleChildEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningOwnerBranchFacts {
    pub(crate) path: PathBuf,
    pub(crate) owner_namespace: Vec<String>,
    pub(crate) roles: Vec<RustReasoningOwnerBranchRole>,
    pub(crate) declared_child_edges: Vec<RustModuleChildEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustReasoningOwnerBranchRole {
    Root,
    Facade,
    Interface,
    Binary,
    PackageEntrypoint,
    RepeatedNamespace(Vec<String>),
    Branch,
}

impl RustReasoningTreeFacts {
    pub(crate) fn module(&self, path: &Path) -> Option<&RustReasoningModuleFacts> {
        self.modules.iter().find(|module| module.path == path)
    }
}

pub(crate) fn rust_reasoning_tree_facts(
    scope: &RustProjectHarnessScope,
    modules: &[ParsedRustModule],
) -> RustReasoningTreeFacts {
    let module_tree = rust_module_tree_facts(&scope.source_paths, modules);
    let source_files = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, &scope.source_paths))
        .map(|module| module.report.path.clone())
        .collect::<BTreeSet<_>>();
    let module_facts = modules
        .iter()
        .map(|module| {
            let is_source_module = source_files.contains(&module.report.path);
            RustReasoningModuleFacts {
                path: module.report.path.clone(),
                source_path: rust_source_path_facts(
                    &scope.project_root,
                    &scope.source_paths,
                    &scope.package_paths,
                    &module.report.path,
                ),
                is_source_module,
                is_module_tree_root: is_source_module
                    && is_module_tree_root(&scope.source_paths, &module.report.path),
                declared_child_edges: if is_source_module {
                    external_child_module_edges(module, &source_files)
                } else {
                    Vec::new()
                },
            }
        })
        .collect::<Vec<_>>();
    let owner_branches = owner_branch_facts(&module_facts);
    RustReasoningTreeFacts {
        package_root: scope.project_root.clone(),
        source_roots: scope.source_paths.clone(),
        package_entrypoints: scope.package_paths.clone(),
        modules: module_facts,
        owner_branches,
        shadowed_module_sources: module_tree.shadowed_module_sources,
        unreachable_source_files: module_tree.unreachable_source_files,
    }
}

fn owner_branch_facts(modules: &[RustReasoningModuleFacts]) -> Vec<RustReasoningOwnerBranchFacts> {
    let mut branches = modules
        .iter()
        .filter(|module| module.is_source_module)
        .filter(|module| {
            module.is_module_tree_root
                || !module.declared_child_edges.is_empty()
                || module.source_path.is_special_entrypoint
                || !module.source_path.repeated_namespace_segments.is_empty()
        })
        .map(|module| RustReasoningOwnerBranchFacts {
            path: module.path.clone(),
            owner_namespace: module.source_path.namespace_components.clone(),
            roles: owner_branch_roles(module),
            declared_child_edges: module.declared_child_edges.clone(),
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        right
            .roles
            .contains(&RustReasoningOwnerBranchRole::Root)
            .cmp(&left.roles.contains(&RustReasoningOwnerBranchRole::Root))
            .then_with(|| left.path.cmp(&right.path))
    });
    branches
}

fn owner_branch_roles(module: &RustReasoningModuleFacts) -> Vec<RustReasoningOwnerBranchRole> {
    let mut roles = Vec::new();
    if module.is_module_tree_root {
        roles.push(RustReasoningOwnerBranchRole::Root);
    }
    if module.source_path.is_crate_facade {
        roles.push(RustReasoningOwnerBranchRole::Facade);
    }
    if module.source_path.is_interface_mod {
        roles.push(RustReasoningOwnerBranchRole::Interface);
    }
    if module.source_path.is_binary_entrypoint {
        roles.push(RustReasoningOwnerBranchRole::Binary);
    }
    if module.source_path.is_package_entrypoint {
        roles.push(RustReasoningOwnerBranchRole::PackageEntrypoint);
    }
    if !module.source_path.repeated_namespace_segments.is_empty() {
        roles.push(RustReasoningOwnerBranchRole::RepeatedNamespace(
            module
                .source_path
                .repeated_namespace_segments
                .iter()
                .cloned()
                .collect(),
        ));
    }
    if roles.is_empty() {
        roles.push(RustReasoningOwnerBranchRole::Branch);
    }
    roles
}

fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}
