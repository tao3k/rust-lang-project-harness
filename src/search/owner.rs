use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::RustProjectHarnessScope;
use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::parser::{ParsedRustModule, parse_rust_file};

use super::RustSearchOptions;
use super::context::{
    PackageSearchContext, exact_owner_path_matches, exact_rust_file_query, search_contexts,
    search_contexts_for_path_query,
};
use super::format::{
    append_block, compact_locations, display_project_path, package_label,
    package_roots_for_request, render_cargo_dependency_line, render_item_line,
    render_public_api_line,
};
use super::hits::{OwnerHit, dependency_usage, matching_dependencies};
use super::limits::{SEARCH_HIT_LIMIT, SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT, SEARCH_TEST_LIMIT};
use super::scope::{module_is_scope, owner_path_matches};

pub(super) fn render_search_dependency(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let block = render_search_dependency_block(project_root, &context, query, options);
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

struct DependencyPipeFlags {
    items: bool,
    docs: bool,
    tests: bool,
    public_api: bool,
}

impl DependencyPipeFlags {
    fn from_options(options: &RustSearchOptions) -> Self {
        let docs = has_pipe(options, "docs") || has_pipe(options, "docs-use");
        Self {
            items: has_pipe(options, "items"),
            docs,
            tests: has_pipe(options, "tests"),
            public_api: has_pipe(options, "public-api") || docs,
        }
    }
}

fn render_search_dependency_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> String {
    let flags = DependencyPipeFlags::from_options(options);
    let deps = matching_dependencies(&context.cargo_dependencies, query);
    let usage = dependency_usage(context, query);
    let owner_modules = owner_modules_for_hits(context, &usage);
    let public_api = dependency_public_api_lines(context, query, &usage, flags.public_api);
    let item_count = dependency_item_count(&owner_modules, flags.items);
    let test_lines = dependency_test_lines(context, &owner_modules, flags.tests);
    let test_count = rendered_test_count(&test_lines);
    let mut block = dependency_header(
        project_root,
        context,
        query,
        DependencyHeaderCounts {
            deps: deps.len(),
            owners: usage.len(),
            api: public_api.len(),
            items: item_count,
            tests: test_count,
        },
        &flags,
    );
    append_dependency_facts(&mut block, &deps);
    append_dependency_owner_hits(&mut block, context, query, &usage, &owner_modules, &flags);
    append_limited_lines(&mut block, public_api, SEARCH_ITEM_LIMIT);
    append_limited_lines(&mut block, test_lines, SEARCH_TEST_LIMIT);
    let _ = writeln!(block, "|next deps:{query},import:{query},tests");
    block
}

struct DependencyHeaderCounts {
    deps: usize,
    owners: usize,
    api: usize,
    items: usize,
    tests: usize,
}

fn dependency_header(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    counts: DependencyHeaderCounts,
    flags: &DependencyPipeFlags,
) -> String {
    let mut block = format!(
        "[search-dependency] q={} pkg={} dep={} own={} api={}",
        query,
        package_label(project_root, &context.package_root),
        counts.deps,
        counts.owners,
        counts.api
    );
    if flags.items {
        let _ = write!(block, " item={}", counts.items);
    }
    if flags.docs {
        let _ = write!(block, " docs={}", counts.api);
    }
    if flags.tests {
        let _ = write!(block, " tests={}", counts.tests);
    }
    block.push('\n');
    block
}

fn dependency_public_api_lines(
    context: &PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    enabled: bool,
) -> Vec<String> {
    if enabled {
        public_api_lines_for_dependency(context, query, usage, None)
    } else {
        Vec::new()
    }
}

fn dependency_item_count(owner_modules: &[&ParsedRustModule], enabled: bool) -> usize {
    if enabled {
        owner_modules
            .iter()
            .map(|module| named_top_level_item_count(module))
            .sum()
    } else {
        0
    }
}

fn dependency_test_lines(
    context: &PackageSearchContext,
    owner_modules: &[&ParsedRustModule],
    enabled: bool,
) -> Vec<String> {
    if enabled {
        test_lines_for_owner_modules(context, owner_modules)
    } else {
        Vec::new()
    }
}

fn rendered_test_count(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with("|test "))
        .count()
}

fn append_dependency_facts(block: &mut String, deps: &[&crate::parser::CargoDependencyFacts]) {
    for dependency in deps.iter().take(SEARCH_HIT_LIMIT) {
        let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
    }
}

fn append_dependency_owner_hits(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    owner_modules: &[&ParsedRustModule],
    flags: &DependencyPipeFlags,
) {
    for hit in usage.iter().take(SEARCH_OWNER_LIMIT) {
        append_dependency_owner_hit(block, context, query, hit, owner_modules, flags);
    }
}

fn append_dependency_owner_hit(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    hit: &OwnerHit,
    owner_modules: &[&ParsedRustModule],
    flags: &DependencyPipeFlags,
) {
    let owner_path = display_project_path(&context.package_root, &hit.path);
    let _ = writeln!(
        block,
        "|owner {} hit_kind=dependency locations={} next=tests",
        owner_path,
        compact_locations(&hit.locations)
    );
    let _ = writeln!(block, "|edge O:{owner_path} -ext:{query}-> D:{query}");
    if flags.items {
        append_hit_item_lines(block, hit, owner_modules);
    }
}

fn append_hit_item_lines(block: &mut String, hit: &OwnerHit, owner_modules: &[&ParsedRustModule]) {
    if let Some(module) = owner_modules
        .iter()
        .find(|module| module.report.path == hit.path)
    {
        append_item_lines(block, module);
    }
}

fn append_limited_lines(block: &mut String, lines: Vec<String>, limit: usize) {
    for line in lines.into_iter().take(limit) {
        let _ = writeln!(block, "{line}");
    }
}

pub(super) fn render_search_tests(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if let Some(query) = query
        && let Some(rendered) = render_exact_path_tests(project_root, config, query, options)?
    {
        return Ok(rendered);
    }
    let contexts = query.map_or_else(
        || search_contexts(project_root, config, options),
        |query| search_contexts_for_path_query(project_root, config, options, query),
    )?;
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
        let block = render_search_tests_block(
            project_root,
            &context.package_root,
            query.unwrap_or("-"),
            &owner_modules,
            tests,
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_exact_path_tests(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    if !exact_rust_file_query(query) {
        return Ok(None);
    }
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let matches = exact_owner_path_matches(project_root, &package_roots, query);
    if matches.is_empty() {
        return Ok(None);
    }

    let mut rendered = String::new();
    for (package_root, path) in matches {
        let scope = rust_project_harness_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let owner_module = parse_rust_file(&path);
        if module_is_scope(&scope, &owner_module, "tests") {
            return Ok(None);
        }
        let test_modules = parse_test_scope(&scope, config);
        let owner_modules = [&owner_module];
        let owner_tokens = test_subject_tokens(&owner_modules);
        let tests = test_modules
            .iter()
            .filter_map(|module| {
                test_match(&package_root, module, Some(query), true, &owner_tokens)
            })
            .collect::<Vec<_>>();
        let block =
            render_search_tests_block(project_root, &package_root, query, &owner_modules, tests);
        append_block(&mut rendered, &block);
    }
    Ok(Some(rendered))
}

fn parse_test_scope(
    scope: &RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<ParsedRustModule> {
    discover_rust_files(&scope.test_paths, &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn render_search_tests_block(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    owner_modules: &[&ParsedRustModule],
    tests: Vec<TestSearchMatch>,
) -> String {
    let mut block = format!(
        "[search-tests] q={} pkg={} tests={} own={}\n",
        query,
        package_label(project_root, package_root),
        tests.len(),
        owner_modules.len()
    );
    for module in owner_modules.iter().take(SEARCH_OWNER_LIMIT) {
        let owner_path = display_project_path(package_root, &module.report.path);
        let _ = writeln!(
            block,
            "|node O:{owner_path} kind=owner path={owner_path} next=owner:{owner_path}"
        );
    }
    for test in tests.into_iter().take(SEARCH_TEST_LIMIT) {
        append_test_lines(&mut block, &test);
    }
    block
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
) -> Vec<&'a ParsedRustModule> {
    context
        .parsed_modules
        .iter()
        .filter(|module| !module_is_scope(&context.scope, module, "tests"))
        .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
        .collect()
}

fn test_subject_tokens(modules: &[&ParsedRustModule]) -> Vec<(PathBuf, String)> {
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

pub(super) fn test_lines_for_owner_modules(
    context: &super::context::PackageSearchContext,
    owner_modules: &[&ParsedRustModule],
) -> Vec<String> {
    let owner_tokens = public_test_subject_tokens(owner_modules);
    if owner_tokens.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for module in context
        .parsed_modules
        .iter()
        .filter(|module| module_is_scope(&context.scope, module, "tests"))
    {
        let Some((owner_path, reasons)) =
            test_owner_symbol_reasons(&context.package_root, &module.source, &owner_tokens)
        else {
            continue;
        };
        let mut rendered = String::new();
        append_test_lines(
            &mut rendered,
            &TestSearchMatch {
                path: display_project_path(&context.package_root, &module.report.path),
                functions: module.syntax_facts.test_function_count,
                owner_path: Some(owner_path),
                reasons,
            },
        );
        lines.extend(rendered.lines().map(ToOwned::to_owned));
    }
    lines
}

fn public_test_subject_tokens(modules: &[&ParsedRustModule]) -> Vec<(PathBuf, String)> {
    let mut seen = BTreeSet::new();
    modules
        .iter()
        .flat_map(|module| public_module_test_tokens(module))
        .filter(|(path, token)| token.len() > 1 && seen.insert((path.clone(), token.clone())))
        .collect()
}

fn public_module_test_tokens(module: &ParsedRustModule) -> Vec<(PathBuf, String)> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public)
        .flat_map(|item| item.name.iter().chain(item.function_name.iter()))
        .map(|token| (module.report.path.clone(), token.clone()))
        .collect()
}

fn test_match(
    package_root: &Path,
    module: &ParsedRustModule,
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

fn owner_modules_for_hits<'a>(
    context: &'a super::context::PackageSearchContext,
    hits: &[OwnerHit],
) -> Vec<&'a ParsedRustModule> {
    let owner_paths = hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    context
        .parsed_modules
        .iter()
        .filter(|module| owner_paths.contains(&module.report.path))
        .collect()
}

fn named_top_level_item_count(module: &ParsedRustModule) -> usize {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some())
        .count()
}

fn append_item_lines(block: &mut String, module: &ParsedRustModule) {
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

fn append_test_lines(block: &mut String, test: &TestSearchMatch) {
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

fn has_pipe(options: &RustSearchOptions, pipe: &str) -> bool {
    options.pipes.iter().any(|candidate| candidate == pipe)
}

pub(super) fn public_api_lines_for_dependency(
    context: &super::context::PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    api_filter: Option<&str>,
) -> Vec<String> {
    let api_filter = api_filter.map(ToOwned::to_owned);
    usage
        .iter()
        .flat_map(|hit| {
            let api_filter = api_filter.clone();
            context
                .parsed_modules
                .iter()
                .filter(move |module| module.report.path == hit.path)
                .flat_map(move |module| {
                    let api_filter = api_filter.clone();
                    module
                        .syntax_facts
                        .top_level_items
                        .iter()
                        .filter_map(move |item| {
                            if !item.is_public
                                || !public_item_matches_api_filter(
                                    module,
                                    item,
                                    api_filter.as_deref(),
                                )
                            {
                                return None;
                            }
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

fn public_item_matches_api_filter(
    module: &ParsedRustModule,
    item: &crate::parser::RustTopLevelItemSyntax,
    api_filter: Option<&str>,
) -> bool {
    let Some(api_filter) = api_filter else {
        return true;
    };
    item.name.as_deref() == Some(api_filter)
        || item.function_name.as_deref() == Some(api_filter)
        || item_context_mentions_api(&module.source, item.line, api_filter)
}

fn item_context_mentions_api(source: &str, line: usize, api_filter: &str) -> bool {
    let line_index = line.saturating_sub(1);
    source
        .lines()
        .skip(line_index.saturating_sub(2))
        .take(3)
        .any(|line| line.contains(api_filter))
}
