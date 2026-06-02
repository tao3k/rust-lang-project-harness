//! Fuzzy lexical search rendering.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::discovery::{discover_rust_files, rust_project_harness_scope};

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    append_block, compact_locations, display_project_path, owner_role_for_path, package_label,
    package_roots_for_request, query_set_terms, sort_locations,
};
use super::limits::SEARCH_HIT_LIMIT;
use super::recency::compare_paths_by_recency;
use super::scope::{module_allowed, path_allowed_by_scope};

pub(super) fn render_search_fzf(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if super::syntax_query::is_rust_syntax_query(query) {
        return Ok(format!(
            "[search-fzf] q={} pkg=. skipped=code-shaped-query\n|query intent=code-shaped status=skipped reason=use-native-syntax-api next=search-query\n",
            query
        ));
    }
    let query_terms = query_set_terms(query);
    if query_terms.len() > 1 {
        return render_search_fzf_query_set(project_root, config, query, &query_terms, options);
    }
    let token_terms: Vec<&str> = query.split_whitespace().collect();
    if token_terms.len() > 1 && !fzf_exact(&options.fzf_args) {
        return render_search_fzf_query_set(project_root, config, query, &token_terms, options);
    }
    if options.output_view.as_deref() == Some("seeds") {
        return render_search_fzf_seed_hits(project_root, config, query, options);
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let hits = fzf_hits(&context, query, options);
        let mut block = format!(
            "[search-fzf] q={} mode={} backend=provider pkg={} own={}{}\n",
            query,
            fzf_match_mode(options),
            package_label(project_root, &context.package_root),
            hits.len(),
            fzf_header_suffix(options)
        );
        for (path, score, locations) in hits.iter().take(SEARCH_HIT_LIMIT) {
            let owner_path = display_project_path(&context.package_root, path);
            let _ = writeln!(
                block,
                "|owner {owner_path} hit_kind=fzf score={} locations={} next=owner:{owner_path}",
                score,
                compact_locations(locations),
            );
        }
        append_change_frontier_synthesis_line(
            &mut block,
            &context.package_root,
            hits.iter()
                .take(SEARCH_HIT_LIMIT)
                .map(|(path, _, _)| path.as_path()),
            SEARCH_HIT_LIMIT,
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_fzf_query_set(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    if options.output_view.as_deref() == Some("seeds") {
        return render_search_fzf_query_set_seed_hits(
            project_root,
            config,
            query,
            query_terms,
            options,
        );
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let hits = fzf_query_set_hits(&context, query_terms, options);
        let mut block = format!(
            "[search-fzf] q={} querySet={} selector=fuzzy-set mode={} backend=provider pkg={} own={}{}\n",
            query,
            query_terms.len(),
            fzf_match_mode(options),
            package_label(project_root, &context.package_root),
            hits.len(),
            fzf_header_suffix(options)
        );
        for (path, terms, score, locations) in hits.iter().take(SEARCH_HIT_LIMIT) {
            let owner_path = display_project_path(&context.package_root, path);
            let _ = writeln!(
                block,
                "|owner {owner_path} hit_kind=fzf querySet={} terms={} score={} locations={} next=owner:{owner_path}",
                terms.len(),
                terms.iter().cloned().collect::<Vec<_>>().join(","),
                score,
                compact_locations(locations),
            );
        }
        append_change_frontier_synthesis_line(
            &mut block,
            &context.package_root,
            hits.iter()
                .take(SEARCH_HIT_LIMIT)
                .map(|(path, _, _, _)| path.as_path()),
            SEARCH_HIT_LIMIT,
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn fzf_query_set_hits(
    context: &PackageSearchContext,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Vec<(PathBuf, BTreeSet<String>, usize, Vec<String>)> {
    let mut grouped = BTreeMap::<PathBuf, (BTreeSet<String>, usize, Vec<String>)>::new();
    for module in context
        .parsed_modules
        .iter()
        .filter(|module| module_allowed(context, module, options))
    {
        let owner_path = display_project_path(&context.package_root, &module.report.path);
        for term in query_terms {
            let Some(score) = fuzzy_score_with_options(&owner_path, term, options) else {
                continue;
            };
            let entry = grouped
                .entry(module.report.path.clone())
                .or_insert_with(|| (BTreeSet::new(), 0, Vec::new()));
            entry.0.insert((*term).to_string());
            entry.1 = entry.1.saturating_add(score);
            entry.2.push("path:1".to_string());
        }
        for (index, line) in module.source.lines().enumerate() {
            for term in query_terms {
                let Some(score) = fuzzy_score_with_options(line, term, options) else {
                    continue;
                };
                let entry = grouped
                    .entry(module.report.path.clone())
                    .or_insert_with(|| (BTreeSet::new(), 0, Vec::new()));
                entry.0.insert((*term).to_string());
                entry.1 = entry.1.saturating_add(score);
                entry.2.push(format!("{}:1", index + 1));
            }
        }
    }
    let mut hits = grouped
        .into_iter()
        .map(|(path, (terms, score, mut locations))| {
            sort_locations(&mut locations);
            locations.dedup();
            (path, terms, score, locations)
        })
        .collect::<Vec<_>>();
    hits.sort_by(|(left, _, left_score, _), (right, _, right_score, _)| {
        right_score
            .cmp(left_score)
            .then_with(|| compare_paths_by_recency(&context.package_root, left, right))
    });
    hits
}

fn render_search_fzf_seed_hits(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in package_roots {
        let hits = fzf_seed_hits(&package_root, config, query, options);
        let seed_limit = options.seeds.unwrap_or(8);
        let owner_limit = seed_limit.min(hits.len());
        let mut block = format!(
            "[search-fzf] q={} mode={} backend=provider pkg={} own={}{}\n",
            query,
            fzf_match_mode(options),
            package_label(project_root, &package_root),
            hits.len(),
            fzf_header_suffix(options)
        );
        let owners = hits
            .iter()
            .take(owner_limit)
            .map(|(path, _, _)| display_project_path(&package_root, path))
            .collect::<Vec<_>>();
        if !owners.is_empty() {
            let _ = writeln!(block, "|seed owner:{}", owners.join(","));
        }
        append_change_frontier_synthesis_line(
            &mut block,
            &package_root,
            hits.iter()
                .take(owner_limit)
                .map(|(path, _, _)| path.as_path()),
            seed_limit,
        );
        if hits.len() > owner_limit {
            let _ = writeln!(
                block,
                "|note seeds_truncated={} limit={}",
                hits.len() - owner_limit,
                seed_limit
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_fzf_query_set_seed_hits(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in package_roots {
        let hits = fzf_query_set_seed_hits(&package_root, config, query_terms, options);
        let seed_limit = options.seeds.unwrap_or(8);
        let owner_limit = seed_limit.min(hits.len());
        let mut block = format!(
            "[search-fzf] q={} querySet={} selector=fuzzy-set mode={} backend=provider pkg={} own={}{}\n",
            query,
            query_terms.len(),
            fzf_match_mode(options),
            package_label(project_root, &package_root),
            hits.len(),
            fzf_header_suffix(options)
        );
        let selected = hits
            .iter()
            .take(owner_limit)
            .map(|(path, _, _, _)| display_project_path(&package_root, path))
            .collect::<Vec<_>>();
        render_query_set_graph(&mut block, selected);
        if hits.len() > owner_limit {
            let _ = writeln!(
                block,
                "|note seeds_truncated={} limit={}",
                hits.len() - owner_limit,
                seed_limit
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_query_set_graph(block: &mut String, selected: Vec<String>) {
    let nodes = selected
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let is_test = path.starts_with("tests/") || path.contains("/tests/");
            let (prefix, kind, action) = if is_test {
                ("T", "test", "tests")
            } else {
                ("O", "owner", "owner")
            };
            (format!("{prefix}{}", index + 1), kind, action, path)
        })
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return;
    }
    let _ = writeln!(
        block,
        "[search-graph] mode=query-set root=. alg=change-frontier-query-set"
    );
    let node_lines = nodes
        .iter()
        .map(|(id, kind, action, path)| format!("{id}={kind}:{path}!{action}"))
        .collect::<Vec<_>>()
        .join("\n");
    let rank = nodes
        .iter()
        .map(|(id, _, _, _)| id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let frontier = nodes
        .iter()
        .map(|(id, _, action, _)| format!("{id}.{action}"))
        .collect::<Vec<_>>()
        .join(",");
    let _ = writeln!(block, "{node_lines}");
    let _ = writeln!(block, "rank={rank}");
    let _ = writeln!(block, "frontier={frontier}");
}

fn append_change_frontier_synthesis_line<'a>(
    block: &mut String,
    package_root: &Path,
    paths: impl IntoIterator<Item = &'a Path>,
    limit: usize,
) {
    let mut seen = BTreeSet::new();
    let mut edit_frontier = Vec::new();
    let mut test_frontier = Vec::new();
    for path in paths {
        let display_path = display_project_path(package_root, path);
        if !seen.insert(display_path.clone()) {
            continue;
        }
        if owner_role_for_path(package_root, path) == "test" {
            if test_frontier.len() < limit {
                test_frontier.push(display_path);
            }
        } else if edit_frontier.len() < limit {
            edit_frontier.push(display_path);
        }
    }
    if edit_frontier.is_empty() && test_frontier.is_empty() {
        return;
    }
    let window_set = edit_frontier
        .iter()
        .map(|path| format!("owner:{path}"))
        .chain(test_frontier.iter().map(|path| format!("tests:{path}")))
        .collect::<Vec<_>>();
    let mut parts = vec![
        "algorithm=change-frontier-query-set".to_string(),
        "scope=query-set".to_string(),
        "summary=query-set-frontier".to_string(),
        format!(
            "selected_owners={}",
            edit_frontier.len() + test_frontier.len()
        ),
    ];
    if !edit_frontier.is_empty() {
        parts.push(format!("edit_frontier={}", edit_frontier.join(",")));
    }
    if !test_frontier.is_empty() {
        parts.push(format!("test_frontier={}", test_frontier.join(",")));
    }
    if !window_set.is_empty() {
        parts.push(format!("window_set={}", window_set.join(",")));
        parts.push(format!("seeds={}", window_set.join(",")));
    }
    let _ = writeln!(block, "|synthesis {}", parts.join(" "));
}

fn fzf_query_set_seed_hits(
    package_root: &Path,
    config: &RustHarnessConfig,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Vec<(PathBuf, BTreeSet<String>, usize, Vec<String>)> {
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let mut hits = discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .filter(|path| path_allowed_by_scope(&scope, package_root, path, options))
        .filter_map(|path| {
            let Ok(text) = fs::read_to_string(&path) else {
                return None;
            };
            let (terms, score, locations) =
                fuzzy_query_set_locations_with_path(package_root, &path, &text, query_terms);
            (!terms.is_empty()).then_some((path, terms, score, locations))
        })
        .collect::<Vec<_>>();
    hits.sort_by(|(left, _, left_score, _), (right, _, right_score, _)| {
        right_score
            .cmp(left_score)
            .then_with(|| compare_paths_by_recency(package_root, left, right))
    });
    hits
}

fn fzf_seed_hits(
    package_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<(PathBuf, usize, Vec<String>)> {
    let scope = rust_project_harness_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let mut hits = discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .filter(|path| path_allowed_by_scope(&scope, package_root, path, options))
        .filter_map(|path| {
            let Ok(text) = fs::read_to_string(&path) else {
                return None;
            };
            let (score, locations) =
                fuzzy_locations_with_path(package_root, &path, &text, query, options);
            (score > 0).then_some((path, score, locations))
        })
        .collect::<Vec<_>>();
    hits.sort_by(|(left, left_score, _), (right, right_score, _)| {
        right_score
            .cmp(left_score)
            .then_with(|| compare_paths_by_recency(package_root, left, right))
    });
    hits
}

fn fzf_hits(
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> Vec<(PathBuf, usize, Vec<String>)> {
    let mut hits = context
        .parsed_modules
        .iter()
        .filter(|module| module_allowed(context, module, options))
        .filter_map(|module| {
            let (score, locations) = fuzzy_locations_with_path(
                &context.package_root,
                &module.report.path,
                &module.source,
                query,
                options,
            );
            (score > 0).then_some((module.report.path.clone(), score, locations))
        })
        .collect::<Vec<_>>();
    hits.sort_by(|(left, left_score, _), (right, right_score, _)| {
        right_score
            .cmp(left_score)
            .then_with(|| compare_paths_by_recency(&context.package_root, left, right))
    });
    hits
}

fn fzf_header_suffix(options: &RustSearchOptions) -> String {
    if options.fzf_args.is_empty() {
        return String::new();
    }
    format!(" finder=fzf fzfArgs={}", options.fzf_args.join(","))
}

fn fzf_match_mode(options: &RustSearchOptions) -> &'static str {
    if fzf_exact(&options.fzf_args) {
        "exact"
    } else {
        "fuzzy"
    }
}

fn fzf_exact(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--exact" | "-e"))
}

fn fzf_case_sensitive(args: &[String], _query: &str) -> bool {
    if args.iter().any(|arg| arg == "+i") {
        return true;
    }
    false
}

fn fuzzy_locations(text: &str, query: &str, options: &RustSearchOptions) -> (usize, Vec<String>) {
    let mut score = 0usize;
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(line_score) = fuzzy_score_with_options(line, query, options) else {
            continue;
        };
        score = score.saturating_add(line_score);
        locations.push(format!("{}:1", index + 1));
    }
    sort_locations(&mut locations);
    locations.dedup();
    (score, locations)
}

fn fuzzy_locations_with_path(
    package_root: &Path,
    path: &Path,
    text: &str,
    query: &str,
    options: &RustSearchOptions,
) -> (usize, Vec<String>) {
    let (mut score, mut locations) = fuzzy_locations(text, query, options);
    let owner_path = display_project_path(package_root, path);
    if let Some(path_score) = fuzzy_score_with_options(&owner_path, query, options) {
        score = score.saturating_add(path_score);
        locations.push("path:1".to_string());
    }
    locations.dedup();
    (score, locations)
}

fn fuzzy_query_set_locations(
    text: &str,
    query_terms: &[&str],
) -> (BTreeSet<String>, usize, Vec<String>) {
    let mut terms = BTreeSet::new();
    let mut score = 0usize;
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for term in query_terms {
            let Some(line_score) = fuzzy_score(line, term) else {
                continue;
            };
            terms.insert((*term).to_string());
            score = score.saturating_add(line_score);
            locations.push(format!("{}:1", index + 1));
        }
    }
    sort_locations(&mut locations);
    locations.dedup();
    (terms, score, locations)
}

fn fuzzy_query_set_locations_with_path(
    package_root: &Path,
    path: &Path,
    text: &str,
    query_terms: &[&str],
) -> (BTreeSet<String>, usize, Vec<String>) {
    let (mut terms, mut score, mut locations) = fuzzy_query_set_locations(text, query_terms);
    let owner_path = display_project_path(package_root, path);
    for term in query_terms {
        let Some(path_score) = fuzzy_score(&owner_path, term) else {
            continue;
        };
        terms.insert((*term).to_string());
        score = score.saturating_add(path_score);
        if !locations.iter().any(|location| location == "path:1") {
            locations.push("path:1".to_string());
        }
    }
    (terms, score, locations)
}

fn fuzzy_score(candidate: &str, query: &str) -> Option<usize> {
    fuzzy_score_with_args(candidate, query, &[])
}

fn fuzzy_score_with_options(
    candidate: &str,
    query: &str,
    options: &RustSearchOptions,
) -> Option<usize> {
    fuzzy_score_with_args(candidate, query, &options.fzf_args)
}

fn fuzzy_score_with_args(candidate: &str, query: &str, fzf_args: &[String]) -> Option<usize> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let exact = fzf_exact(fzf_args);
    let case_sensitive = fzf_case_sensitive(fzf_args, query);
    let candidate = if case_sensitive {
        candidate.to_string()
    } else {
        candidate.to_ascii_lowercase()
    };
    let query = if case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    };
    if let Some(index) = candidate.find(&query) {
        return Some(
            10_000usize
                .saturating_add(query.len().saturating_mul(100))
                .saturating_sub(index.min(1_000)),
        );
    }
    if exact {
        return None;
    }
    let positions = fuzzy_match_positions(&candidate, &query)?;
    if positions.is_empty() {
        return None;
    }
    let first = *positions.first()?;
    let last = *positions.last()?;
    let span = last.saturating_sub(first).saturating_add(1);
    if span
        > query
            .len()
            .saturating_mul(3)
            .max(query.len().saturating_add(12))
    {
        return None;
    }
    let compactness_penalty = span.saturating_sub(positions.len()).min(2_000);
    Some(
        5_000usize
            .saturating_add(positions.len().saturating_mul(50))
            .saturating_sub(compactness_penalty)
            .saturating_sub(first.min(1_000)),
    )
}

fn fuzzy_match_positions(candidate: &str, query: &str) -> Option<Vec<usize>> {
    query
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .try_fold((Vec::new(), 0usize), |(mut positions, cursor), ch| {
            let suffix = candidate.get(cursor..)?;
            let offset = suffix.find(ch)?;
            let position = cursor + offset;
            positions.push(position);
            Some((positions, position + ch.len_utf8()))
        })
        .map(|(positions, _)| positions)
}
