use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::parser::{ParsedRustModule, RustReasoningOwnerBranchFacts};

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    display_project_path, owner_role_for_path, package_label, render_item_line, render_owner_line,
};
use super::limits::{SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT};
use super::scope::{owner_branch_matches, owner_path_matches};

pub(super) fn render_search_owner(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    Ok(contexts
        .iter()
        .map(|context| render_search_owner_block(project_root, context, query, options))
        .collect())
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
        let path = display_project_path(&context.package_root, &module.report.path);
        let role = owner_role_for_path(&context.package_root, &module.report.path);
        let _ = writeln!(
            block,
            "|owner {path} role={role} public=false source=parser-visible-module parserOwner=false layer={role} lines={} valid={} syntaxDiagnostics={} semanticDiagnostics=0 imports={} next=owner:{path},text:{path}(owner={path}),tests:{path}",
            module.source.lines().count(),
            module.report.is_valid,
            usize::from(module.report.parse_error.is_some()),
            module_import_count(module)
        );
    }
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
    } else {
        None
    }
}
