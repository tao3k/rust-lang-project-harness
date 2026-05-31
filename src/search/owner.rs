use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;

use super::RustSearchOptions;
use super::context::search_contexts;
use super::format::{
    append_block, compact_locations, display_project_path, package_label,
    render_cargo_dependency_line, render_item_line, render_owner_line, render_public_api_line,
};
use super::hits::{OwnerHit, dependency_usage, matching_dependencies};
use super::limits::{SEARCH_HIT_LIMIT, SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT, SEARCH_TEST_LIMIT};
use super::scope::{module_is_scope, owner_branch_matches, owner_path_matches};

pub(super) fn render_search_owner(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let include_items = options.pipes.iter().any(|pipe| pipe == "items");
        let matching_branches = context
            .reasoning_tree
            .owner_branches
            .iter()
            .filter(|branch| owner_branch_matches(&context.package_root, branch, query))
            .collect::<Vec<_>>();
        let matching_modules = context
            .parsed_modules
            .iter()
            .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
            .collect::<Vec<_>>();
        let mut block = format!(
            "[search-owner] q={} pkg={} own={} item={}\n",
            query,
            package_label(project_root, &context.package_root),
            matching_branches.len().max(matching_modules.len()),
            if include_items {
                matching_modules
                    .iter()
                    .map(|module| module.syntax_facts.top_level_items.len())
                    .sum()
            } else {
                0
            }
        );
        for branch in matching_branches.iter().take(SEARCH_OWNER_LIMIT) {
            let module = context
                .parsed_modules
                .iter()
                .find(|module| module.report.path == branch.path);
            let _ = writeln!(
                block,
                "{}",
                render_owner_line(&context.package_root, branch, module)
            );
        }
        if include_items {
            for module in matching_modules.iter().take(SEARCH_OWNER_LIMIT) {
                for item in module
                    .syntax_facts
                    .top_level_items
                    .iter()
                    .filter(|item| item.name.is_some())
                    .take(SEARCH_ITEM_LIMIT)
                {
                    let _ = writeln!(block, "{}", render_item_line(item));
                }
            }
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_dependency(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let deps = matching_dependencies(&context.cargo_dependencies, query);
        let usage = dependency_usage(&context, query);
        let public_api = if options.pipes.iter().any(|pipe| pipe == "public-api") {
            public_api_lines_for_dependency(&context, query, &usage)
        } else {
            Vec::new()
        };
        let mut block = format!(
            "[search-dependency] q={} pkg={} dep={} own={} api={}\n",
            query,
            package_label(project_root, &context.package_root),
            deps.len(),
            usage.len(),
            public_api.len()
        );
        for dependency in deps.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        for hit in usage.into_iter().take(SEARCH_OWNER_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hit_kind=dependency locations={} next=tests",
                display_project_path(&context.package_root, &hit.path),
                compact_locations(&hit.locations)
            );
        }
        for line in public_api.into_iter().take(SEARCH_ITEM_LIMIT) {
            let _ = writeln!(block, "{line}");
        }
        let _ = writeln!(block, "|next deps:{query},import:{query},tests");
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_tests(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let owner_modules = query
            .map(|query| test_subject_modules(&context, query))
            .unwrap_or_default();
        let owner_tokens = test_subject_tokens(&owner_modules);
        let tests = context
            .parsed_modules
            .iter()
            .filter(|module| module_is_scope(&context.scope, module, "tests"))
            .filter_map(|module| {
                test_match(
                    &context.package_root,
                    module,
                    query,
                    !owner_modules.is_empty(),
                    &owner_tokens,
                )
            })
            .collect::<Vec<_>>();
        let mut block = format!(
            "[search-tests] q={} pkg={} tests={} own={}\n",
            query.unwrap_or("-"),
            package_label(project_root, &context.package_root),
            tests.len(),
            owner_modules.len()
        );
        for module in owner_modules.iter().take(SEARCH_OWNER_LIMIT) {
            let owner_path = display_project_path(&context.package_root, &module.report.path);
            let _ = writeln!(
                block,
                "|node O:{owner_path} kind=owner path={owner_path} next=owner:{owner_path}"
            );
        }
        for test in tests.into_iter().take(SEARCH_TEST_LIMIT) {
            let _ = writeln!(
                block,
                "|test {} functions={}{} next=owner:{}",
                test.path,
                test.functions,
                test.metadata(),
                test.path
            );
            if let Some(owner_path) = test.owner_path.as_deref() {
                let _ = writeln!(block, "|edge O:{owner_path} -test-> T:{}", test.path);
            }
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

struct TestSearchMatch {
    path: String,
    functions: usize,
    owner_path: Option<String>,
    reasons: Vec<String>,
}

impl TestSearchMatch {
    fn metadata(&self) -> String {
        let mut parts = Vec::new();
        if let Some(owner_path) = self.owner_path.as_deref() {
            parts.push(format!("owner={owner_path}"));
        }
        if !self.reasons.is_empty() {
            parts.push(format!("reason={}", self.reasons.join(",")));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" {}", parts.join(" "))
        }
    }
}

fn test_subject_modules<'a>(
    context: &'a super::context::PackageSearchContext,
    query: &str,
) -> Vec<&'a crate::parser::ParsedRustModule> {
    context
        .parsed_modules
        .iter()
        .filter(|module| !module_is_scope(&context.scope, module, "tests"))
        .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
        .collect()
}

fn test_subject_tokens(modules: &[&crate::parser::ParsedRustModule]) -> Vec<(PathBuf, String)> {
    let mut seen = BTreeSet::new();
    let mut tokens = Vec::new();
    for module in modules {
        for item in &module.syntax_facts.top_level_items {
            for token in item.name.iter().chain(item.function_name.iter()) {
                if token.len() > 1 && seen.insert((module.report.path.clone(), token.clone())) {
                    tokens.push((module.report.path.clone(), token.clone()));
                }
            }
        }
    }
    tokens
}

fn test_match(
    package_root: &Path,
    module: &crate::parser::ParsedRustModule,
    query: Option<&str>,
    has_owner_query: bool,
    owner_tokens: &[(PathBuf, String)],
) -> Option<TestSearchMatch> {
    let path = display_project_path(package_root, &module.report.path);
    let Some(query) = query else {
        return Some(TestSearchMatch {
            path,
            functions: module.syntax_facts.test_function_count,
            owner_path: None,
            reasons: Vec::new(),
        });
    };
    if owner_path_matches(package_root, &module.report.path, query)
        || (!has_owner_query && module.source.contains(query))
    {
        return Some(TestSearchMatch {
            path,
            functions: module.syntax_facts.test_function_count,
            owner_path: None,
            reasons: Vec::new(),
        });
    }
    let (owner_path, reasons) =
        test_owner_symbol_reasons(package_root, &module.source, owner_tokens)?;
    Some(TestSearchMatch {
        path,
        functions: module.syntax_facts.test_function_count,
        owner_path: Some(owner_path),
        reasons,
    })
}

fn test_owner_symbol_reasons(
    package_root: &Path,
    source: &str,
    owner_tokens: &[(PathBuf, String)],
) -> Option<(String, Vec<String>)> {
    let (owner_path, _) = owner_tokens
        .iter()
        .find(|(_, token)| source.contains(token))?;
    let reasons = owner_tokens
        .iter()
        .filter(|(candidate_path, token)| candidate_path == owner_path && source.contains(token))
        .map(|(_, token)| format!("symbol:{token}"))
        .collect::<Vec<_>>();
    Some((display_project_path(package_root, owner_path), reasons))
}

pub(super) fn public_api_lines_for_dependency(
    context: &super::context::PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
) -> Vec<String> {
    usage
        .iter()
        .flat_map(|hit| {
            context
                .parsed_modules
                .iter()
                .filter(move |module| module.report.path == hit.path)
                .flat_map(move |module| {
                    module
                        .syntax_facts
                        .top_level_items
                        .iter()
                        .filter(|item| item.is_public)
                        .filter_map(move |item| {
                            render_public_api_line(
                                &context.package_root,
                                &module.report.path,
                                query,
                                item,
                            )
                        })
                })
        })
        .collect()
}
