//! Project reasoning-tree facts derived from parsed Rust modules.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::RustProjectHarnessScope;

use super::module_tree::{
    RustModuleSourceShadow, external_child_module_paths, is_module_tree_root,
    rust_module_tree_facts,
};
use super::{ParsedRustModule, RustSourcePathFacts, rust_source_path_facts};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustReasoningTreeFacts {
    pub(crate) package_root: PathBuf,
    pub(crate) source_roots: Vec<PathBuf>,
    pub(crate) package_entrypoints: Vec<PathBuf>,
    pub(crate) modules: Vec<RustReasoningModuleFacts>,
    pub(crate) shadowed_module_sources: Vec<RustModuleSourceShadow>,
    pub(crate) unreachable_source_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningModuleFacts {
    pub(crate) path: PathBuf,
    pub(crate) source_path: RustSourcePathFacts,
    pub(crate) is_source_module: bool,
    pub(crate) is_module_tree_root: bool,
    pub(crate) declared_child_paths: Vec<PathBuf>,
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
                declared_child_paths: if is_source_module {
                    external_child_module_paths(module, &source_files)
                } else {
                    Vec::new()
                },
            }
        })
        .collect();
    RustReasoningTreeFacts {
        package_root: scope.project_root.clone(),
        source_roots: scope.source_paths.clone(),
        package_entrypoints: scope.package_paths.clone(),
        modules: module_facts,
        shadowed_module_sources: module_tree.shadowed_module_sources,
        unreachable_source_files: module_tree.unreachable_source_files,
    }
}

fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}
