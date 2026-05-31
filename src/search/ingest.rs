//! RFC `search ingest` renderer for grouping external candidate streams.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;

use super::RustSearchOptions;
use super::format::{
    compact_locations, display_project_path, owner_role_for_path, package_roots_for_request,
};
use super::limits::SEARCH_OWNER_LIMIT;

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
    let mut owner_hits = BTreeMap::<PathBuf, Vec<String>>::new();
    for (path, location) in candidates {
        for package_root in &package_roots {
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
    let mut rendered = format!(
        "[search-ingest] src={} in={} own={}\n",
        source.as_str(),
        input.lines().count(),
        owner_hits.len()
    );
    for (owner, locations) in owner_hits.into_iter().take(SEARCH_OWNER_LIMIT) {
        let package_root = package_roots
            .iter()
            .find(|package_root| owner.starts_with(package_root))
            .map_or(project_root, PathBuf::as_path);
        let mut line = format!(
            "|owner {} role={} hit_kind={} locations={}",
            display_project_path(package_root, &owner),
            owner_role_for_path(package_root, &owner),
            source.hit_kind(),
            compact_locations(&locations)
        );
        line.push_str(" next=owner");
        let _ = writeln!(rendered, "{line}");
    }
    Ok(rendered)
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
