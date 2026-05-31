//! RFC `search ingest` renderer for grouping external candidate streams.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::parser::{ParsedRustModule, RustTopLevelItemSyntax};

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    compact_locations, display_project_path, owner_role_for_path, package_roots_for_request,
    render_item_line,
};
use super::limits::{SEARCH_OWNER_LIMIT, SEARCH_TEST_LIMIT};
use super::owner::test_lines_for_owner_modules;

/// Render grouped search candidates from external tool output.
///
/// # Errors
///
/// Returns an error when the project root or selected package cannot be
/// resolved.
pub fn render_rust_project_harness_search_ingest_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
    input: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let source = detect_ingest_source(input);
    let candidates = ingest_candidates(input, source);
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let contexts = ingest_pipe_contexts(project_root, config, options)?;
    let owner_hits = grouped_owner_hits(candidates, &package_roots);
    Ok(render_ingest_owner_hits(
        project_root,
        input,
        source,
        &package_roots,
        &contexts,
        &owner_hits,
        options,
    ))
}

fn ingest_pipe_contexts(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<Vec<PackageSearchContext>, String> {
    let include_items = has_pipe(options, "items");
    let include_tests = has_pipe(options, "tests");
    if include_items || include_tests {
        search_contexts(project_root, config, options)
    } else {
        Ok(Vec::new())
    }
}

fn grouped_owner_hits(
    candidates: Vec<(PathBuf, Vec<String>)>,
    package_roots: &[PathBuf],
) -> BTreeMap<PathBuf, Vec<String>> {
    let mut owner_hits = BTreeMap::<PathBuf, Vec<String>>::new();
    for (path, location) in candidates {
        for package_root in package_roots {
            let absolute = if path.is_absolute() {
                path.clone()
            } else {
                package_root.join(&path)
            };
            if absolute.exists() {
                owner_hits
                    .entry(absolute)
                    .or_default()
                    .extend(location.clone());
                break;
            }
        }
    }
    owner_hits
}

fn render_ingest_owner_hits(
    project_root: &Path,
    input: &str,
    source: IngestSource,
    package_roots: &[PathBuf],
    contexts: &[PackageSearchContext],
    owner_hits: &BTreeMap<PathBuf, Vec<String>>,
    options: &RustSearchOptions,
) -> String {
    let include_items = has_pipe(options, "items");
    let include_tests = has_pipe(options, "tests");
    let mut rendered = format!(
        "[search-ingest] src={} in={} own={}\n",
        source.as_str(),
        input.lines().count(),
        owner_hits.len()
    );
    for (owner, locations) in owner_hits.iter().take(SEARCH_OWNER_LIMIT) {
        let package_root = package_roots
            .iter()
            .find(|package_root| owner.starts_with(package_root))
            .map_or(project_root, PathBuf::as_path);
        let mut line = format!(
            "|owner {} role={} hit_kind={} locations={}",
            display_project_path(package_root, owner),
            owner_role_for_path(package_root, owner),
            source.hit_kind(),
            compact_locations(locations)
        );
        line.push_str(" next=owner");
        let _ = writeln!(rendered, "{line}");
        append_ingest_pipe_lines(
            &mut rendered,
            contexts,
            owner,
            locations,
            include_items,
            include_tests,
        );
    }
    rendered
}

fn append_ingest_pipe_lines(
    rendered: &mut String,
    contexts: &[PackageSearchContext],
    owner: &Path,
    locations: &[String],
    include_items: bool,
    include_tests: bool,
) {
    if !include_items && !include_tests {
        return;
    }
    let Some((context, module)) = context_module_for_owner(contexts, owner) else {
        return;
    };
    if include_items {
        for line in ingest_item_lines(module, locations) {
            let _ = writeln!(rendered, "{line}");
        }
    }
    if include_tests {
        for line in test_lines_for_owner_modules(context, &[module])
            .into_iter()
            .take(SEARCH_TEST_LIMIT)
        {
            let _ = writeln!(rendered, "{line}");
        }
    }
}

fn context_module_for_owner<'a>(
    contexts: &'a [PackageSearchContext],
    owner: &Path,
) -> Option<(&'a PackageSearchContext, &'a ParsedRustModule)> {
    contexts.iter().find_map(|context| {
        let module = context
            .parsed_modules
            .iter()
            .find(|module| module.report.path == owner)?;
        Some((context, module))
    })
}

fn ingest_item_lines(module: &ParsedRustModule, locations: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    locations
        .iter()
        .filter_map(|location| location_line_number(location))
        .filter_map(|line| nearest_item_for_line(module, line))
        .filter(|item| {
            seen.insert((
                item.line,
                item.name
                    .as_deref()
                    .or(item.function_name.as_deref())
                    .unwrap_or("-")
                    .to_string(),
            ))
        })
        .map(render_item_line)
        .collect()
}

fn nearest_item_for_line(
    module: &ParsedRustModule,
    line: usize,
) -> Option<&RustTopLevelItemSyntax> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some() || item.function_name.is_some())
        .filter(|item| item.line <= line)
        .max_by_key(|item| item.line)
        .or_else(|| {
            module
                .syntax_facts
                .top_level_items
                .iter()
                .filter(|item| item.name.is_some() || item.function_name.is_some())
                .min_by_key(|item| item.line)
        })
}

fn location_line_number(location: &str) -> Option<usize> {
    location.split_once(':')?.0.parse::<usize>().ok()
}

fn has_pipe(options: &RustSearchOptions, pipe: &str) -> bool {
    options.pipes.iter().any(|candidate| candidate == pipe)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IngestSource {
    RgN,
    PathList,
    DiffPaths,
    Unknown,
}

impl IngestSource {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RgN => "rg-n",
            Self::PathList => "paths",
            Self::DiffPaths => "diff-paths",
            Self::Unknown => "unknown",
        }
    }

    const fn hit_kind(self) -> &'static str {
        match self {
            Self::RgN => "text",
            Self::PathList | Self::DiffPaths => "path",
            Self::Unknown => "unknown",
        }
    }
}

fn detect_ingest_source(input: &str) -> IngestSource {
    let first = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    if first.starts_with("diff --git ") {
        return IngestSource::DiffPaths;
    }
    if parse_rg_line(first).is_some() {
        return IngestSource::RgN;
    }
    if !first.is_empty() {
        return IngestSource::PathList;
    }
    IngestSource::Unknown
}

fn ingest_candidates(input: &str, source: IngestSource) -> Vec<(PathBuf, Vec<String>)> {
    match source {
        IngestSource::RgN => input
            .lines()
            .filter_map(parse_rg_line)
            .map(|(path, line)| (path, vec![format!("{line}:1")]))
            .collect(),
        IngestSource::DiffPaths => input
            .lines()
            .filter_map(|line| line.strip_prefix("diff --git a/"))
            .filter_map(|line| line.split_once(" b/").map(|(_, right)| right))
            .map(|path| (PathBuf::from(path), Vec::new()))
            .collect(),
        IngestSource::PathList | IngestSource::Unknown => input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| (PathBuf::from(line), Vec::new()))
            .collect(),
    }
}

fn parse_rg_line(line: &str) -> Option<(PathBuf, usize)> {
    let mut parts = line.splitn(3, ':');
    let path = parts.next()?;
    let line_number = parts.next()?.parse::<usize>().ok()?;
    Some((PathBuf::from(path), line_number))
}
