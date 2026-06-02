//! Parser-owned item query matching and compact code lines.

use std::fs;
use std::path::Path;

use crate::parser::{ParsedRustModule, RustTopLevelItemSyntax};

use super::format::{display_project_path, render_item_line_with_read};
use super::limits::{SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT};

pub(super) fn owner_item_count(
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
    item_query: Option<&str>,
) -> usize {
    if !include_items {
        return 0;
    }
    matching_modules
        .iter()
        .flat_map(|module| module_items_for_query(module, item_query))
        .count()
}

pub(super) fn module_items_for_query<'a>(
    module: &'a ParsedRustModule,
    item_query: Option<&str>,
) -> Vec<&'a RustTopLevelItemSyntax> {
    let named_items = named_module_items(module);
    let Some(query) = item_query.map(str::trim).filter(|query| !query.is_empty()) else {
        return named_items;
    };
    let terms = item_query_terms(query);
    let exact = named_items
        .iter()
        .copied()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_exact(item, term))
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }
    named_items
        .into_iter()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_fuzzy(item, term))
        })
        .collect()
}

pub(super) fn render_owner_item_lines(
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
    names_only: bool,
) -> Vec<String> {
    matching_modules
        .iter()
        .take(SEARCH_OWNER_LIMIT)
        .flat_map(|module| render_module_item_lines(package_root, module, item_query, names_only))
        .collect()
}

pub(super) fn render_owner_item_code_lines(
    _package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
) -> Vec<String> {
    matching_modules
        .iter()
        .take(SEARCH_OWNER_LIMIT)
        .flat_map(|module| {
            let Ok(source) = fs::read_to_string(&module.report.path) else {
                return Vec::new();
            };
            module_items_for_query(module, item_query)
                .into_iter()
                .take(SEARCH_ITEM_LIMIT)
                .filter_map(|item| render_item_source_text(&source, item))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn render_item_query_line(
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
    names_only: bool,
) -> Option<String> {
    let query = item_query
        .map(str::trim)
        .filter(|query| !query.is_empty())?;
    let summary = item_query_match_summary(matching_modules, query);
    let output = names_only.then_some(" output=names").unwrap_or_default();
    let candidate_field = if summary.candidates.is_empty() {
        String::new()
    } else {
        format!(" candidates={}", summary.candidates.join(","))
    };
    let next = if summary.mode == ItemQueryMatchMode::Miss {
        summary.next.as_deref().unwrap_or("revise-query")
    } else {
        "code"
    };
    Some(format!(
        "|query itemQuery={} status={} match={} item={} reason={}{}{} next={}",
        query,
        summary.mode.status(),
        summary.mode.label(),
        summary.item_count,
        summary.mode.reason(),
        output,
        candidate_field,
        next
    ))
}

fn render_module_item_lines(
    package_root: &Path,
    module: &ParsedRustModule,
    item_query: Option<&str>,
    names_only: bool,
) -> Vec<String> {
    module_items_for_query(module, item_query)
        .into_iter()
        .take(SEARCH_ITEM_LIMIT)
        .flat_map(|item| render_item_lines(package_root, module, item, item_query, names_only))
        .collect()
}

fn render_item_lines(
    package_root: &Path,
    module: &ParsedRustModule,
    item: &RustTopLevelItemSyntax,
    item_query: Option<&str>,
    names_only: bool,
) -> Vec<String> {
    let mut lines = vec![render_item_line_with_read(
        package_root,
        &module.report.path,
        item,
    )];
    if item_query.is_some()
        && !names_only
        && let Some(line) = render_item_code_line(package_root, module, item)
    {
        lines.push(line);
    }
    lines
}

fn render_item_code_line(
    package_root: &Path,
    module: &ParsedRustModule,
    item: &RustTopLevelItemSyntax,
) -> Option<String> {
    let path = display_project_path(package_root, &module.report.path);
    let start_line = item.line.max(1);
    let end_line = item.end_line.max(start_line);
    let text = item_projection_text(item);
    if text.is_empty() {
        return None;
    }
    let truncated = item.projection_nodes.len() > 48;
    let encoded_text = serde_json::to_string(&text).ok()?;
    Some(format!(
        "|code path={} startLine={} endLine={} reason=item-query truncated={} text={}",
        path, start_line, end_line, truncated, encoded_text
    ))
}

fn render_item_source_text(source: &str, item: &RustTopLevelItemSyntax) -> Option<String> {
    let start_line = item.line.max(1);
    let end_line = item.end_line.max(start_line);
    let text = source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemQueryMatchMode {
    Exact,
    FallbackContains,
    Miss,
}

struct ItemQuerySummary {
    mode: ItemQueryMatchMode,
    item_count: usize,
    candidates: Vec<String>,
    next: Option<String>,
}

impl ItemQueryMatchMode {
    fn status(self) -> &'static str {
        match self {
            Self::Exact | Self::FallbackContains => "hit",
            Self::Miss => "miss",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::FallbackContains => "fallback-contains",
            Self::Miss => "none",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Exact => "parser-item-exact",
            Self::FallbackContains => "parser-item-fallback",
            Self::Miss => "parser-item-miss",
        }
    }
}

fn item_query_match_summary(
    matching_modules: &[&ParsedRustModule],
    query: &str,
) -> ItemQuerySummary {
    let terms = item_query_terms(query);
    let named_items = matching_modules
        .iter()
        .flat_map(|module| named_module_items(module))
        .collect::<Vec<_>>();
    let exact_count = matching_modules
        .iter()
        .flat_map(|module| named_module_items(module))
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_exact(item, term))
        })
        .count();
    if exact_count > 0 {
        return ItemQuerySummary {
            mode: ItemQueryMatchMode::Exact,
            item_count: exact_count,
            candidates: Vec::new(),
            next: None,
        };
    }
    let fuzzy_count = named_items
        .iter()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_fuzzy(item, term))
        })
        .count();
    if fuzzy_count > 0 {
        ItemQuerySummary {
            mode: ItemQueryMatchMode::FallbackContains,
            item_count: fuzzy_count,
            candidates: Vec::new(),
            next: None,
        }
    } else {
        let candidates = item_query_miss_candidates(&named_items, &terms);
        let next = candidates
            .first()
            .map(|candidate| format!("query:{candidate}"));
        ItemQuerySummary {
            mode: ItemQueryMatchMode::Miss,
            item_count: 0,
            candidates,
            next,
        }
    }
}

fn item_query_miss_candidates(items: &[&RustTopLevelItemSyntax], terms: &[&str]) -> Vec<String> {
    let mut scored = items
        .iter()
        .filter_map(|item| {
            let name = item_query_candidate_name(item)?;
            let score = terms
                .iter()
                .map(|term| item_query_candidate_score(name, term))
                .max()
                .unwrap_or(0);
            (score > 0).then(|| (score, name.to_string()))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left_name), (right_score, right_name)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_name.cmp(right_name))
    });
    let mut seen = std::collections::HashSet::new();
    scored
        .into_iter()
        .map(|(_, name)| name)
        .filter(|name| seen.insert(name.clone()))
        .take(5)
        .collect()
}

fn item_query_candidate_name(item: &RustTopLevelItemSyntax) -> Option<&str> {
    item.name
        .as_deref()
        .or(item.function_name.as_deref())
        .or(item.macro_name.as_deref())
        .or(item.include_target.as_deref())
}

fn item_query_candidate_score(candidate: &str, term: &str) -> usize {
    let candidate = normalize_identifier(candidate);
    let term = normalize_identifier(term);
    if candidate.is_empty() || term.is_empty() {
        return 0;
    }
    if candidate == term {
        return 100;
    }
    if candidate.contains(&term) || term.contains(&candidate) {
        return 80;
    }
    if let Some(prefix) = term.rsplit_once('_').map(|(prefix, _)| prefix)
        && prefix.len() >= 4
        && candidate.starts_with(prefix)
    {
        return 70 + prefix.len().min(20);
    }
    let shared_tokens = shared_identifier_token_count(&candidate, &term);
    if shared_tokens >= 2 {
        return 40 + shared_tokens * 5;
    }
    let common_prefix = common_prefix_len(&candidate, &term);
    if common_prefix >= 4 {
        return 20 + common_prefix.min(20);
    }
    0
}

fn normalize_identifier(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

fn shared_identifier_token_count(left: &str, right: &str) -> usize {
    let left_tokens = identifier_tokens(left);
    let right_tokens = identifier_tokens(right);
    left_tokens
        .iter()
        .filter(|token| right_tokens.iter().any(|right| right == *token))
        .count()
}

fn identifier_tokens(value: &str) -> Vec<&str> {
    value.split('_').filter(|token| token.len() >= 2).collect()
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn named_module_items(module: &ParsedRustModule) -> Vec<&RustTopLevelItemSyntax> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some())
        .collect()
}

fn item_query_terms(query: &str) -> Vec<&str> {
    query
        .split('|')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect()
}

fn item_matches_query_exact(item: &RustTopLevelItemSyntax, query: &str) -> bool {
    item_query_candidates(item)
        .into_iter()
        .flatten()
        .any(|candidate| candidate == query)
}

fn item_matches_query_fuzzy(item: &RustTopLevelItemSyntax, query: &str) -> bool {
    item_query_candidates(item)
        .into_iter()
        .flatten()
        .any(|candidate| candidate.contains(query))
}

fn item_query_candidates(item: &RustTopLevelItemSyntax) -> [Option<&str>; 5] {
    [
        item.name.as_deref(),
        item.function_name.as_deref(),
        item.macro_name.as_deref(),
        item.include_target.as_deref(),
        Some(item.kind),
    ]
}

fn item_projection_text(item: &RustTopLevelItemSyntax) -> String {
    item.projection_nodes
        .iter()
        .map(|node| node.label.trim())
        .filter(|line| !line.is_empty())
        .take(48)
        .collect::<Vec<_>>()
        .join("\n")
}
