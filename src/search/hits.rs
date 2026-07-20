use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::CargoDependencyFacts;

use super::RustSearchOptions;
use super::context::PackageSearchContext;
use super::format::{display_project_path, sort_locations};
use super::recency::compare_paths_by_recency;
use super::scope::module_allowed;

#[derive(Debug, Clone)]
pub(super) struct SearchHit {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) kind: String,
    pub(super) name: String,
}

impl SearchHit {
    pub(super) fn render(&self, package_root: &Path) -> String {
        format!(
            "{}:{} kind={} name={} next=owner:{}",
            display_project_path(package_root, &self.path),
            self.line,
            self.kind,
            self.name,
            display_project_path(package_root, &self.path)
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct OwnerHit {
    pub(super) path: PathBuf,
    pub(super) locations: Vec<String>,
}

pub(super) fn symbol_definitions(
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<SearchHit> {
    let mut hits = context
        .parsed_modules
        .iter()
        .filter(|module| module_allowed(context, module, options))
        .flat_map(|module| {
            let mut module_hits = module
                .syntax_facts
                .top_level_items
                .iter()
                .filter(move |item| {
                    item.name.as_deref() == Some(query)
                        || item.function_name.as_deref() == Some(query)
                })
                .map(move |item| SearchHit {
                    path: module.report.path.clone(),
                    line: item.line,
                    kind: item.kind.to_string(),
                    name: item
                        .name
                        .clone()
                        .or_else(|| item.function_name.clone())
                        .unwrap_or_else(|| query.to_string()),
                })
                .collect::<Vec<_>>();
            module_hits.extend(
                module
                    .syntax_facts
                    .public_api_callables
                    .iter()
                    .filter(move |callable| {
                        callable.kind == "method"
                            && !callable.is_test_context
                            && callable.name == query
                    })
                    .map(move |callable| SearchHit {
                        path: module.report.path.clone(),
                        line: callable.line,
                        kind: callable.kind.to_string(),
                        name: callable.name.clone(),
                    }),
            );
            module_hits
        })
        .collect::<Vec<_>>();
    sort_search_hits_by_recency(&context.package_root, &mut hits);
    hits.dedup_by(|left, right| {
        left.path == right.path
            && left.line == right.line
            && left.kind == right.kind
            && left.name == right.name
    });
    hits
}

pub(super) fn symbol_calls(
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    for module in context
        .parsed_modules
        .iter()
        .filter(|module| module_allowed(context, module, options))
    {
        hits.extend(
            module
                .syntax_facts
                .function_calls
                .iter()
                .filter(|call| call.terminal_name == query)
                .map(|call| SearchHit {
                    path: module.report.path.clone(),
                    line: call.line,
                    kind: "call".to_string(),
                    name: query.to_string(),
                }),
        );
        hits.extend(
            module
                .syntax_facts
                .path_references
                .iter()
                .filter(|reference| reference.terminal_name == query)
                .map(|reference| SearchHit {
                    path: module.report.path.clone(),
                    line: reference.line,
                    kind: "ref".to_string(),
                    name: query.to_string(),
                }),
        );
    }
    hits.sort_by(|left, right| {
        compare_paths_by_recency(&context.package_root, &left.path, &right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    hits.dedup_by(|left, right| {
        left.path == right.path && left.line == right.line && left.kind == right.kind
    });
    hits
}

pub(super) fn import_hits(
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<OwnerHit> {
    grouped_owner_hits(
        context,
        context
            .parsed_modules
            .iter()
            .filter(|module| module_allowed(context, module, options))
            .flat_map(|module| {
                module
                    .syntax_facts
                    .use_statements
                    .iter()
                    .filter(move |use_statement| {
                        use_statement.imports.iter().any(|import| {
                            import.segments.iter().any(|segment| segment == query)
                                || import.exposed_name.as_deref() == Some(query)
                        })
                    })
                    .map(move |use_statement| {
                        (
                            module.report.path.clone(),
                            format!("{}:1", use_statement.line),
                        )
                    })
            }),
    )
}

pub(super) fn text_hits(
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<OwnerHit> {
    grouped_owner_hits(
        context,
        context
            .parsed_modules
            .iter()
            .filter(|module| module_allowed(context, module, options))
            .flat_map(|module| {
                module
                    .source
                    .lines()
                    .enumerate()
                    .filter(move |(_, line)| line.contains(query))
                    .map(move |(index, _)| (module.report.path.clone(), format!("{}:1", index + 1)))
            }),
    )
}

pub(super) fn dependency_usage(context: &PackageSearchContext, query: &str) -> Vec<OwnerHit> {
    let mut names = BTreeSet::from([query.to_string(), query.replace('-', "_")]);
    for dependency in matching_dependencies(&context.cargo_dependencies, query) {
        names.insert(dependency.dependency_key.clone());
        names.insert(dependency.import_name.clone());
        names.insert(dependency.package_name.clone());
    }
    grouped_owner_hits(
        context,
        context.parsed_modules.iter().flat_map(|module| {
            let names = names.clone();
            module
                .syntax_facts
                .use_statements
                .iter()
                .filter(move |use_statement| {
                    use_statement.imports.iter().any(|import| {
                        import
                            .segments
                            .first()
                            .is_some_and(|segment| names.contains(segment))
                    })
                })
                .map(move |use_statement| {
                    (
                        module.report.path.clone(),
                        format!("{}:1", use_statement.line),
                    )
                })
        }),
    )
}

fn grouped_owner_hits(
    context: &PackageSearchContext,
    hits: impl IntoIterator<Item = (PathBuf, String)>,
) -> Vec<OwnerHit> {
    let mut grouped = BTreeMap::<PathBuf, Vec<String>>::new();
    for (path, location) in hits {
        grouped.entry(path).or_default().push(location);
    }
    let mut owner_hits = grouped
        .into_iter()
        .map(|(path, mut locations)| {
            sort_locations(&mut locations);
            locations.dedup();
            OwnerHit { path, locations }
        })
        .collect::<Vec<_>>();
    sort_owner_hits_by_recency(&context.package_root, &mut owner_hits);
    owner_hits
}

pub(super) fn sort_owner_hits_by_recency(package_root: &Path, hits: &mut [OwnerHit]) {
    hits.sort_by(|left, right| compare_paths_by_recency(package_root, &left.path, &right.path));
}

pub(super) fn sort_search_hits_by_recency(package_root: &Path, hits: &mut [SearchHit]) {
    hits.sort_by(|left, right| {
        compare_paths_by_recency(package_root, &left.path, &right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });
}

pub(super) fn matching_dependencies<'a>(
    dependencies: &'a [CargoDependencyFacts],
    query: &str,
) -> Vec<&'a CargoDependencyFacts> {
    let import_query = query.replace('-', "_");
    dependencies
        .iter()
        .filter(|dependency| {
            dependency.dependency_key == query
                || dependency.import_name == query
                || dependency.import_name == import_query
                || dependency.package_name == query
        })
        .collect()
}
