use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, RustReasoningTreeFacts, parse_cargo_dependency_facts,
    parse_rust_file, rust_reasoning_tree_facts,
};
use crate::{RustHarnessConfig, RustProjectHarnessScope};

use super::RustSearchOptions;
use super::format::{package_label, package_roots_for_request};

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

pub(super) fn search_contexts_for_path_query(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
    query: &str,
) -> Result<Vec<PackageSearchContext>, String> {
    search_contexts_for_path_queries(project_root, config, options, &[query])
}

pub(super) fn search_contexts_for_path_queries(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
    queries: &[&str],
) -> Result<Vec<PackageSearchContext>, String> {
    package_roots_for_request(project_root, config, options.package.as_deref()).map(|roots| {
        let roots = if options.package.is_some() {
            roots
        } else {
            path_query_package_roots(project_root, roots, queries)
        };
        roots
            .into_iter()
            .map(|package_root| package_search_context(&package_root, config))
            .collect()
    })
}

fn path_query_package_roots(
    project_root: &Path,
    roots: Vec<PathBuf>,
    queries: &[&str],
) -> Vec<PathBuf> {
    if !queries
        .iter()
        .any(|query| path_query_can_select_package(query))
    {
        return roots;
    }
    let matching = roots
        .iter()
        .filter(|package_root| {
            queries
                .iter()
                .any(|query| package_root_matches_path_query(project_root, package_root, query))
        })
        .cloned()
        .collect::<Vec<_>>();
    if matching.is_empty() { roots } else { matching }
}

fn path_query_can_select_package(query: &str) -> bool {
    query.contains('/') || query.contains('\\') || query.ends_with(".rs")
}

fn package_root_matches_path_query(project_root: &Path, package_root: &Path, query: &str) -> bool {
    let query = query.replace('\\', "/");
    let query_path = Path::new(&query);
    if query_path.is_absolute() {
        return query_path.starts_with(package_root);
    }
    let label = package_label(project_root, package_root);
    if label != "." && (query == label || query.starts_with(&format!("{label}/"))) {
        return true;
    }
    project_root.join(query_path).starts_with(package_root)
}

pub(super) fn exact_rust_file_query(query: &str) -> bool {
    query.replace('\\', "/").ends_with(".rs")
}

pub(super) fn exact_owner_path_matches(
    project_root: &Path,
    package_roots: &[PathBuf],
    query: &str,
) -> Vec<(PathBuf, PathBuf)> {
    let query = query.replace('\\', "/");
    let query_path = Path::new(&query);
    package_roots
        .iter()
        .filter_map(|package_root| {
            exact_owner_path_match(project_root, package_root, query_path)
                .map(|path| (package_root.clone(), path))
        })
        .collect()
}

fn exact_owner_path_match(
    project_root: &Path,
    package_root: &Path,
    query_path: &Path,
) -> Option<PathBuf> {
    if query_path.is_absolute() {
        return exact_rust_path_in_package(query_path, package_root);
    }
    [project_root.join(query_path), package_root.join(query_path)]
        .into_iter()
        .find_map(|candidate| exact_rust_path_in_package(&candidate, package_root))
}

fn exact_rust_path_in_package(path: &Path, package_root: &Path) -> Option<PathBuf> {
    path.exists()
        .then_some(path)
        .filter(|path| path.starts_with(package_root))
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        })
        .map(Path::to_path_buf)
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
