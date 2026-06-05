use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, RustReasoningTreeFacts, parse_cargo_dependency_facts,
    parse_rust_file, rust_reasoning_tree_facts,
};
use crate::path::normalize_lexical_path;
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
            .iter()
            .map(|package_root| package_search_context(package_root.as_path(), config))
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
            .iter()
            .map(|package_root| package_search_context(package_root.as_path(), config))
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
    if !matching.is_empty() {
        return matching;
    }
    let direct_package_roots = direct_package_roots_for_path_queries(project_root, queries);
    if direct_package_roots.is_empty() {
        roots
    } else {
        direct_package_roots
    }
}

fn path_query_can_select_package(query: &str) -> bool {
    query.contains('/') || query.contains('\\') || query.ends_with(".rs")
}

fn package_root_matches_path_query(project_root: &Path, package_root: &Path, query: &str) -> bool {
    let query = query.replace('\\', "/");
    let query_path = Path::new(&query);
    let project_root = normalize_lexical_path(project_root);
    let package_root = normalize_lexical_path(package_root);
    if query_path.is_absolute() {
        return normalize_lexical_path(query_path).starts_with(&package_root);
    }
    let label = package_label(&project_root, &package_root);
    if label != "." && (query == label || query.starts_with(&format!("{label}/"))) {
        return true;
    }
    normalize_lexical_path(&project_root.join(query_path)).starts_with(&package_root)
}

fn direct_package_roots_for_path_queries(project_root: &Path, queries: &[&str]) -> Vec<PathBuf> {
    queries
        .iter()
        .filter_map(|query| direct_package_root_for_path_query(project_root, query))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn direct_package_root_for_path_query(project_root: &Path, query: &str) -> Option<PathBuf> {
    if query.contains('*') || query.contains('{') || query.contains('}') {
        return None;
    }
    let query = query.replace('\\', "/");
    let query_path = Path::new(&query);
    let project_root = normalize_lexical_path(project_root);
    let absolute_path = if query_path.is_absolute() {
        query_path.to_path_buf()
    } else {
        project_root.join(query_path)
    };
    let absolute_path = normalize_lexical_path(&absolute_path);
    if !absolute_path.starts_with(&project_root) {
        return None;
    }
    let mut current = if absolute_path.is_file() {
        absolute_path.parent()?
    } else {
        absolute_path.as_path()
    };
    loop {
        if current.join("Cargo.toml").is_file() {
            return Some(current.to_path_buf());
        }
        if current == project_root {
            return None;
        }
        current = current.parent()?;
    }
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
    exact_owner_path_candidates(project_root, package_root, query_path)
        .into_iter()
        .find_map(|candidate| exact_rust_path_in_package(&candidate, package_root))
}

fn exact_owner_path_candidates(
    project_root: &Path,
    package_root: &Path,
    query_path: &Path,
) -> Vec<PathBuf> {
    let mut candidates = vec![project_root.join(query_path), package_root.join(query_path)];
    if let Some(package_relative_path) = strip_package_root_query_prefix(package_root, query_path) {
        candidates.push(package_root.join(package_relative_path));
    }
    candidates
}

fn strip_package_root_query_prefix(package_root: &Path, query_path: &Path) -> Option<PathBuf> {
    if let Some(path) = strip_package_root_query_prefix_once(package_root, query_path) {
        return Some(path);
    }
    if package_root.is_relative()
        && let Ok(current_dir) = std::env::current_dir()
    {
        return strip_package_root_query_prefix_once(&current_dir.join(package_root), query_path);
    }
    None
}

fn strip_package_root_query_prefix_once(package_root: &Path, query_path: &Path) -> Option<PathBuf> {
    let package_components = normal_path_components(package_root);
    let query_components = normal_path_components(query_path);
    if query_components.len() < 2 {
        return None;
    }
    if let Some(path) = strip_workspace_languages_query_prefix(package_root, &query_components) {
        return Some(path);
    }
    for start in 0..package_components.len() {
        let suffix = &package_components[start..];
        if suffix.is_empty() || suffix.len() >= query_components.len() {
            continue;
        }
        if query_components.starts_with(suffix) {
            return Some(query_components[suffix.len()..].iter().collect());
        }
    }
    None
}

fn strip_workspace_languages_query_prefix(
    package_root: &Path,
    query_components: &[std::ffi::OsString],
) -> Option<PathBuf> {
    let package_name = package_root.file_name()?;
    if query_components.first()?.as_os_str() != std::ffi::OsStr::new("languages") {
        return None;
    }
    if query_components.get(1)?.as_os_str() != package_name {
        return None;
    }
    Some(query_components[2..].iter().collect())
}

fn normal_path_components(path: &Path) -> Vec<std::ffi::OsString> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_os_string()),
            _ => None,
        })
        .collect()
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
