use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::parser::{ParsedRustModule, RustReasoningOwnerBranchFacts, parse_rust_file};

use super::RustSearchOptions;
use super::context::{
    PackageSearchContext, exact_owner_path_matches, exact_rust_file_query,
    search_contexts_for_path_queries, search_contexts_for_path_query,
};
use super::format::{
    display_project_path, owner_role_for_path, package_label, package_roots_for_request,
    query_set_terms, render_item_line, render_owner_line,
};
use super::limits::{SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT};
use super::scope::{owner_branch_matches, owner_path_matches};

pub(super) fn render_search_owner(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let query_terms = query_set_terms(query);
    if query_terms.len() > 1 {
        return render_search_owner_query_set(project_root, config, query, &query_terms, options);
    }
    if let Some(rendered) = render_exact_path_owner(project_root, config, query, options)? {
        return Ok(rendered);
    }
    let contexts = search_contexts_for_path_query(project_root, config, options, query)?;
    Ok(contexts
        .iter()
        .map(|context| render_search_owner_block(project_root, context, query, options))
        .collect())
}

fn render_search_owner_query_set(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    if query_terms.iter().all(|term| exact_rust_file_query(term))
        && let Some(rendered) =
            render_exact_path_owner_query_set(project_root, config, query, query_terms, options)?
    {
        return Ok(rendered);
    }
    let contexts = search_contexts_for_path_queries(project_root, config, options, query_terms)?;
    Ok(contexts
        .iter()
        .map(|context| {
            render_search_owner_query_set_block(project_root, context, query, query_terms, options)
        })
        .collect())
}

fn render_exact_path_owner_query_set(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let mut rendered = String::new();
    for package_root in package_roots {
        let modules = query_terms
            .iter()
            .flat_map(|term| exact_owner_path_matches(project_root, &[package_root.clone()], term))
            .map(|(_, path)| parse_rust_file(&path))
            .collect::<Vec<_>>();
        if modules.is_empty() {
            continue;
        }
        let item_count = if include_items {
            modules
                .iter()
                .map(|module| module.syntax_facts.top_level_items.len())
                .sum()
        } else {
            0
        };
        let mut block = format!(
            "[search-owner] q={} querySet={} selector=exact-set pkg={} own={} item={}\n",
            query,
            query_terms.len(),
            package_label(project_root, &package_root),
            modules.len(),
            item_count
        );
        for module in &modules {
            append_parser_visible_owner_line(&mut block, &package_root, module);
        }
        append_owner_item_lines(
            &mut block,
            &modules.iter().collect::<Vec<_>>(),
            include_items,
        );
        rendered.push_str(&block);
    }
    Ok((!rendered.is_empty()).then_some(rendered))
}

fn render_exact_path_owner(
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

    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let mut rendered = String::new();
    for (package_root, path) in matches {
        let module = parse_rust_file(&path);
        rendered.push_str(&render_exact_path_owner_block(
            project_root,
            &package_root,
            query,
            &module,
            include_items,
        ));
    }
    Ok(Some(rendered))
}

fn render_exact_path_owner_block(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    module: &ParsedRustModule,
    include_items: bool,
) -> String {
    let mut block =
        render_exact_path_owner_header(project_root, package_root, query, module, include_items);
    append_parser_visible_owner_line(&mut block, package_root, module);
    append_owner_item_lines(&mut block, &[module], include_items);
    block
}

fn render_search_owner_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> String {
    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let matching_branches = matching_owner_branches(context, query);
    let matching_modules = matching_owner_modules(context, query);
    let mut block = render_search_owner_header(
        project_root,
        context,
        query,
        &matching_branches,
        &matching_modules,
        include_items,
    );
    append_owner_graph_or_fallback_lines(
        &mut block,
        context,
        query,
        &matching_branches,
        &matching_modules,
    );
    append_owner_item_lines(&mut block, &matching_modules, include_items);
    block
}

fn render_search_owner_query_set_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> String {
    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let details = query_terms
        .iter()
        .map(|term| owner_query_details(context, term, include_items))
        .collect::<Vec<_>>();
    let mut block = format!(
        "[search-owner] q={} querySet={} selector=exact-set pkg={} own={} item={}\n",
        query,
        query_terms.len(),
        package_label(project_root, &context.package_root),
        details.iter().map(|detail| detail.owners).sum::<usize>(),
        details.iter().map(|detail| detail.items).sum::<usize>()
    );
    append_unique_lines(
        &mut block,
        details.into_iter().flat_map(|detail| detail.lines),
    );
    block
}

struct OwnerQueryDetails {
    owners: usize,
    items: usize,
    lines: Vec<String>,
}

fn owner_query_details(
    context: &PackageSearchContext,
    query: &str,
    include_items: bool,
) -> OwnerQueryDetails {
    let matching_branches = matching_owner_branches(context, query);
    let matching_modules = matching_owner_modules(context, query);
    let owners = search_owner_count(context, query, &matching_branches, &matching_modules);
    let items = if include_items {
        matching_modules
            .iter()
            .map(|module| module.syntax_facts.top_level_items.len())
            .sum()
    } else {
        0
    };
    let mut lines = String::new();
    append_owner_graph_or_fallback_lines(
        &mut lines,
        context,
        query,
        &matching_branches,
        &matching_modules,
    );
    append_owner_item_lines(&mut lines, &matching_modules, include_items);
    OwnerQueryDetails {
        owners,
        items,
        lines: lines.lines().map(ToOwned::to_owned).collect(),
    }
}

fn append_unique_lines(block: &mut String, lines: impl IntoIterator<Item = String>) {
    let mut seen = std::collections::BTreeSet::new();
    for line in lines {
        if seen.insert(line.clone()) {
            let _ = writeln!(block, "{line}");
        }
    }
}

fn matching_owner_branches<'a>(
    context: &'a PackageSearchContext,
    query: &str,
) -> Vec<&'a RustReasoningOwnerBranchFacts> {
    context
        .reasoning_tree
        .owner_branches
        .iter()
        .filter(|branch| owner_branch_matches(&context.package_root, branch, query))
        .collect()
}

fn matching_owner_modules<'a>(
    context: &'a PackageSearchContext,
    query: &str,
) -> Vec<&'a ParsedRustModule> {
    context
        .parsed_modules
        .iter()
        .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
        .collect()
}

fn append_owner_graph_or_fallback_lines(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
) {
    if matching_branches.is_empty() {
        append_owner_fallback_lines(block, context, query, matching_modules);
    } else {
        append_reasoning_owner_lines(block, context, matching_branches);
    }
}

fn append_reasoning_owner_lines(
    block: &mut String,
    context: &PackageSearchContext,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
) {
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
}

fn append_owner_item_lines(
    block: &mut String,
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
) {
    if !include_items {
        return;
    }
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

fn render_search_owner_header(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
) -> String {
    format!(
        "[search-owner] q={} pkg={} own={} item={}\n",
        query,
        package_label(project_root, &context.package_root),
        search_owner_count(context, query, matching_branches, matching_modules),
        if include_items {
            matching_modules
                .iter()
                .map(|module| module.syntax_facts.top_level_items.len())
                .sum()
        } else {
            0
        }
    )
}

fn render_exact_path_owner_header(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    module: &ParsedRustModule,
    include_items: bool,
) -> String {
    format!(
        "[search-owner] q={} pkg={} own=1 item={}\n",
        query,
        package_label(project_root, package_root),
        if include_items {
            module.syntax_facts.top_level_items.len()
        } else {
            0
        }
    )
}

fn search_owner_count(
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
) -> usize {
    if !matching_branches.is_empty() || !matching_modules.is_empty() {
        return matching_branches.len().max(matching_modules.len());
    }
    usize::from(path_only_owner_path(context, query).is_some())
}

fn append_owner_fallback_lines(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    matching_modules: &[&ParsedRustModule],
) {
    if !matching_modules.is_empty() {
        append_parser_visible_owner_lines(block, context, matching_modules);
    } else if let Some(path) = path_only_owner_path(context, query) {
        append_path_only_owner_line(block, context, &path);
    } else {
        append_owner_not_found_line(block, query);
    }
}

fn append_parser_visible_owner_lines(
    block: &mut String,
    context: &PackageSearchContext,
    matching_modules: &[&ParsedRustModule],
) {
    for module in matching_modules.iter().take(SEARCH_OWNER_LIMIT) {
        append_parser_visible_owner_line(block, &context.package_root, module);
    }
}

fn append_parser_visible_owner_line(
    block: &mut String,
    package_root: &Path,
    module: &ParsedRustModule,
) {
    let path = display_project_path(package_root, &module.report.path);
    let role = owner_role_for_path(package_root, &module.report.path);
    let _ = writeln!(
        block,
        "|owner {path} role={role} public=false source=parser-visible-module parserOwner=false layer={role} lines={} valid={} syntaxDiagnostics={} semanticDiagnostics=0 imports={} next=owner:{path},text:{path}(owner={path}),tests:{path}",
        module.source.lines().count(),
        module.report.is_valid,
        usize::from(module.report.parse_error.is_some()),
        module_import_count(module)
    );
}

fn append_path_only_owner_line(block: &mut String, context: &PackageSearchContext, path: &Path) {
    let path = display_project_path(&context.package_root, path);
    let role = owner_role_for_path(&context.package_root, Path::new(&path));
    let _ = writeln!(
        block,
        "|owner {path} role={role} public=false source=path-only parserOwner=false next=ingest:{path}"
    );
    block.push_str(
        "|note kind=owner-not-found message=\"path exists but is not a parser-visible owner; use search ingest for line evidence\"\n",
    );
}

fn append_owner_not_found_line(block: &mut String, query: &str) {
    let _ = writeln!(
        block,
        "|note kind=not-found message=\"owner not found: {query}\""
    );
    block.push_str("|next prime:search-prime\n");
}

fn module_import_count(module: &ParsedRustModule) -> usize {
    module
        .syntax_facts
        .use_statements
        .iter()
        .map(|statement| statement.imports.len())
        .sum()
}

fn path_only_owner_path(context: &PackageSearchContext, query: &str) -> Option<PathBuf> {
    let query_path = Path::new(query);
    let absolute = if query_path.is_absolute() {
        query_path.to_path_buf()
    } else {
        context.package_root.join(query_path)
    };
    if absolute.exists() {
        Some(absolute)
    } else if query_path.is_absolute() {
        None
    } else {
        context
            .package_root
            .ancestors()
            .map(|ancestor| ancestor.join(query_path))
            .find(|candidate| candidate.exists() && candidate.starts_with(&context.package_root))
    }
}
