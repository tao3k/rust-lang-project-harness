use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use agent_semantic_tree_sitter_runtime::{compile_query_source, execute_query};

use super::predicate::{SyntaxQueryPredicate, native_query_match_predicates_match};
use crate::cli::tree_sitter_query_locator::{SyntaxQuerySelector, syntax_selector_matches};
use crate::cli::tree_sitter_query_packet::SyntaxQueryRow;

pub(in crate::cli) struct SyntaxQueryProjection {
    pub(in crate::cli) rows: Vec<SyntaxQueryRow>,
    pub(in crate::cli) total_matches: usize,
    pub(in crate::cli) truncated: bool,
    pub(in crate::cli) unsupported_predicates: Vec<String>,
    pub(in crate::cli) selected_file_count: usize,
    pub(in crate::cli) parsed_file_count: usize,
    pub(in crate::cli) cursor_match_count: usize,
    pub(in crate::cli) elapsed_ms: u128,
}

impl SyntaxQueryProjection {
    pub(in crate::cli) fn match_status(&self) -> &'static str {
        if !self.unsupported_predicates.is_empty() && self.total_matches == 0 {
            "unsupported"
        } else if self.total_matches == 0 {
            "miss"
        } else {
            "hit"
        }
    }
}

pub(in crate::cli) fn project_native_tree_sitter_query(
    project_root: &Path,
    query_source: &str,
    predicates: &[SyntaxQueryPredicate],
    selector: Option<&SyntaxQuerySelector>,
) -> Result<SyntaxQueryProjection, String> {
    let started = Instant::now();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let query = compile_query_source(&language, query_source)?;
    let unsupported_predicates = query.unsupported_predicates().to_vec();
    let project_root = absolute_query_project_root(project_root);
    let source_files = rust_source_files(&project_root, selector)?;
    let selected_file_count = source_files.len();
    if !unsupported_predicates.is_empty() {
        return Ok(SyntaxQueryProjection {
            rows: Vec::new(),
            total_matches: 0,
            truncated: false,
            unsupported_predicates,
            selected_file_count,
            parsed_file_count: 0,
            cursor_match_count: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let normalized_selector =
        normalized_selector_for_sources(&project_root, selector, &source_files);
    let active_selector = normalized_selector.as_ref().or(selector);
    let mut rows = Vec::new();
    let mut parsed_file_count = 0usize;
    let mut cursor_match_count = 0usize;
    for path in source_files {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let execution = execute_query(&language, &query, &source)?;
        parsed_file_count += usize::from(execution.parsed);
        cursor_match_count += execution.matches.len();
        let normalized_path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let relative_path = query_relative_path(&project_root, &normalized_path);
        for query_match in execution.matches {
            if !native_query_match_predicates_match(&query_match, predicates)? {
                continue;
            }
            for capture in query_match.captures {
                let item = capture
                    .ancestors
                    .iter()
                    .find(|ancestor| native_definition_node(&ancestor.node_kind))
                    .unwrap_or(&capture.node);
                if !syntax_selector_matches(
                    active_selector,
                    &relative_path,
                    capture.node.start_line,
                    capture.node.end_line,
                    item.start_line,
                    item.end_line,
                ) {
                    continue;
                }
                rows.push(SyntaxQueryRow {
                    capture: capture.capture_name.clone(),
                    capture_node: capture.node.node_kind.clone(),
                    capture_field: native_capture_field(&capture.capture_name),
                    capture_text: capture.node.text.clone(),
                    node: item.node_kind.clone(),
                    name: capture.node.text.clone(),
                    path: relative_path.clone(),
                    start_line: capture.node.start_line,
                    end_line: capture.node.end_line,
                    item_start_line: item.start_line,
                    item_end_line: item.end_line,
                    item_code: item.text.clone(),
                });
            }
        }
    }
    let total_matches = rows.len();
    Ok(SyntaxQueryProjection {
        rows,
        total_matches,
        truncated: false,
        unsupported_predicates,
        selected_file_count,
        parsed_file_count,
        cursor_match_count,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn native_definition_node(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "impl_item"
            | "type_item"
            | "const_item"
            | "static_item"
            | "mod_item"
    )
}

fn native_capture_field(capture_name: &str) -> String {
    capture_name
        .rsplit_once('.')
        .map_or_else(|| capture_name.to_string(), |(_, field)| field.to_string())
}

fn rust_source_files(
    project_root: &Path,
    selector: Option<&SyntaxQuerySelector>,
) -> Result<Vec<PathBuf>, String> {
    if let Some(files) = selector_source_files(project_root, selector)? {
        return Ok(files);
    }
    let mut files = Vec::new();
    for root in rust_query_source_roots(project_root) {
        collect_rust_source_files(&root, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn selector_source_files(
    project_root: &Path,
    selector: Option<&SyntaxQuerySelector>,
) -> Result<Option<Vec<PathBuf>>, String> {
    let Some(selector) = selector else {
        return Ok(None);
    };
    let selector_path = selector.path();
    if selector_path
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']'))
    {
        return Ok(None);
    }
    let path = Path::new(selector_path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    if !candidate.exists() {
        return Ok(None);
    }
    let mut files = Vec::new();
    collect_rust_source_files(&candidate, &mut files)?;
    files.sort();
    files.dedup();
    if files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(files))
    }
}

fn collect_rust_source_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|error| {
        format!(
            "failed to read Rust query project root {}: {error}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read Rust query project entry under {}: {error}",
                path.display()
            )
        })?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if should_skip_query_dir(&entry_path) {
                continue;
            }
            collect_rust_source_files(&entry_path, files)?;
        } else if entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs")
        {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn rust_query_source_roots(project_root: &Path) -> Vec<PathBuf> {
    if project_root.is_file() {
        return vec![project_root.to_path_buf()];
    }
    let mut roots = ["src", "tests", "benches", "examples"]
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let build_script = project_root.join("build.rs");
    if build_script.is_file() {
        roots.push(build_script);
    }
    if roots.is_empty() {
        roots.push(project_root.to_path_buf());
    }
    roots
}

fn should_skip_query_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "target" | "vendor" | "dist" | "build" | "result"
        )
}

fn normalized_selector_for_sources(
    project_root: &Path,
    selector: Option<&SyntaxQuerySelector>,
    source_files: &[PathBuf],
) -> Option<SyntaxQuerySelector> {
    let selector = selector?;
    let selector_path = selector.path();
    if selector_path
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']'))
    {
        return None;
    }
    let path = Path::new(selector_path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    let canonical_candidate = fs::canonicalize(&candidate).ok()?;
    if canonical_candidate.is_dir() {
        return Some(SyntaxQuerySelector {
            path: selector.path().to_string(),
            start_line: selector.start_line,
            end_line: selector.end_line,
            matches_all_paths: true,
        });
    }
    source_files.iter().find_map(|source_file| {
        let canonical_source = fs::canonicalize(source_file).ok()?;
        if canonical_source == canonical_candidate {
            Some(SyntaxQuerySelector {
                path: query_relative_path(project_root, &canonical_source),
                start_line: selector.start_line,
                end_line: selector.end_line,
                matches_all_paths: false,
            })
        } else {
            None
        }
    })
}

fn absolute_query_project_root(project_root: &Path) -> PathBuf {
    let absolute = if project_root.is_absolute() {
        project_root.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(project_root)
    };
    fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn query_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
