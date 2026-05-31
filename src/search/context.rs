use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, RustReasoningTreeFacts, parse_cargo_dependency_facts,
    parse_rust_file, rust_reasoning_tree_facts,
};
use crate::{RustHarnessConfig, RustProjectHarnessScope};

use super::RustSearchOptions;
use super::format::package_roots_for_request;

pub(super) fn parse_scope(
    scope: &RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<ParsedRustModule> {
    discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

pub(super) struct PackageSearchContext {
    pub(super) package_root: PathBuf,
    pub(super) scope: RustProjectHarnessScope,
    pub(super) parsed_modules: Vec<ParsedRustModule>,
    pub(super) reasoning_tree: RustReasoningTreeFacts,
    pub(super) cargo_dependencies: Vec<CargoDependencyFacts>,
}

pub(super) fn search_contexts(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<Vec<PackageSearchContext>, String> {
    package_roots_for_request(project_root, config, options.package.as_deref()).map(|roots| {
        roots
            .into_iter()
            .map(|package_root| package_search_context(&package_root, config))
            .collect()
    })
}

fn package_search_context(package_root: &Path, config: &RustHarnessConfig) -> PackageSearchContext {
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let parsed_modules = parse_scope(&scope, config);
    let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
    let cargo_dependencies = parse_cargo_dependency_facts(package_root);
    PackageSearchContext {
        package_root: package_root.to_path_buf(),
        scope,
        parsed_modules,
        reasoning_tree,
        cargo_dependencies,
    }
}
