//! File discovery for conventional Rust project layouts.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::RustProjectHarnessScope;
use crate::parser::parse_cargo_manifest;

/// Directory names ignored by default during recursive discovery.
pub const DEFAULT_IGNORED_DIR_NAMES: &[&str] = &[
    ".cache",
    ".direnv",
    ".git",
    ".idea",
    ".jj",
    ".run",
    ".vscode",
    "node_modules",
    "target",
];

/// Discover Rust source files under files or directories.
#[must_use]
pub fn discover_rust_files(
    paths: &[PathBuf],
    ignored_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();
    for path in paths {
        discover_path(path, ignored_dir_names, &mut files);
    }
    files.into_iter().collect()
}

/// Discover Cargo package roots under a project or workspace root.
#[must_use]
pub(crate) fn discover_cargo_package_roots(
    project_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let manifest_path = project_root.join("Cargo.toml");
    if manifest_path.is_file() {
        return discover_package_roots_from_manifest(project_root, ignored_dir_names);
    }

    let mut manifests = BTreeSet::new();
    discover_cargo_manifests(project_root, ignored_dir_names, &mut manifests);
    manifests
        .into_iter()
        .filter_map(|manifest| manifest.parent().map(Path::to_path_buf))
        .collect()
}

fn discover_path(path: &Path, ignored_dir_names: &BTreeSet<String>, files: &mut BTreeSet<PathBuf>) {
    if should_ignore(path, ignored_dir_names) {
        return;
    }
    if path.is_file() {
        if is_rust_file(path) {
            files.insert(path.to_path_buf());
        }
        return;
    }
    if !path.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        discover_path(&entry.path(), ignored_dir_names, files);
    }
}

fn discover_cargo_manifests(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
    manifests: &mut BTreeSet<PathBuf>,
) {
    if should_ignore(path, ignored_dir_names) {
        return;
    }
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            manifests.insert(path.to_path_buf());
        }
        return;
    }
    if !path.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        discover_cargo_manifests(&entry.path(), ignored_dir_names, manifests);
    }
}

fn should_ignore(path: &Path, ignored_dir_names: &BTreeSet<String>) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| ignored_dir_names.contains(name))
}

fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

fn discover_package_roots_from_manifest(
    project_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let manifest = parse_cargo_manifest(project_root);
    if manifest.workspace_members.is_empty() {
        return vec![project_root.to_path_buf()];
    }

    let excludes = manifest
        .workspace_excludes
        .iter()
        .map(|pattern| project_root.join(pattern))
        .collect::<Vec<_>>();
    let mut roots = BTreeSet::new();
    if manifest.has_package {
        roots.insert(project_root.to_path_buf());
    }
    for member_pattern in manifest.workspace_members {
        for member_root in
            expand_workspace_member_pattern(project_root, &member_pattern, ignored_dir_names)
        {
            if is_excluded_member(&member_root, &excludes) {
                continue;
            }
            if ignored_path_contains_ignored_segment(&member_root, ignored_dir_names) {
                continue;
            }
            if member_root.join("Cargo.toml").is_file() {
                roots.insert(member_root);
            }
        }
    }
    roots.into_iter().collect()
}

fn expand_workspace_member_pattern(
    project_root: &Path,
    pattern: &str,
    ignored_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    if !pattern.contains('*') {
        return vec![project_root.join(pattern)];
    }
    let search_root = fixed_prefix_root(project_root, pattern);
    let mut manifests = BTreeSet::new();
    discover_cargo_manifests(&search_root, ignored_dir_names, &mut manifests);
    manifests
        .into_iter()
        .filter_map(|manifest| manifest.parent().map(Path::to_path_buf))
        .filter(|root| {
            root.strip_prefix(project_root)
                .ok()
                .and_then(Path::to_str)
                .is_some_and(|relative| glob_pattern_matches(pattern, relative))
        })
        .collect()
}

fn fixed_prefix_root(project_root: &Path, pattern: &str) -> PathBuf {
    let before_star = pattern.split('*').next().unwrap_or_default();
    let prefix = before_star
        .rsplit_once('/')
        .map_or("", |(prefix, _)| prefix);
    if prefix.is_empty() {
        return project_root.to_path_buf();
    }
    project_root.join(prefix)
}

fn glob_pattern_matches(pattern: &str, value: &str) -> bool {
    let pattern_components = pattern.split('/').collect::<Vec<_>>();
    let value_components = value.split('/').collect::<Vec<_>>();
    pattern_components.len() == value_components.len()
        && pattern_components.iter().zip(value_components).all(
            |(pattern_component, value_component)| {
                glob_component_matches(pattern_component, value_component)
            },
        )
}

fn glob_component_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let mut remaining = value;
    let mut parts = pattern.split('*').peekable();
    let Some(first) = parts.next() else {
        return pattern == value;
    };
    if !remaining.starts_with(first) {
        return false;
    }
    remaining = &remaining[first.len()..];
    while let Some(part) = parts.next() {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
        if parts.peek().is_none() && !remaining.is_empty() {
            return false;
        }
    }
    pattern.ends_with('*') || remaining.is_empty()
}

fn is_excluded_member(member_root: &Path, excludes: &[PathBuf]) -> bool {
    excludes
        .iter()
        .any(|excluded| member_root == excluded || member_root.starts_with(excluded))
}

fn ignored_path_contains_ignored_segment(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
) -> bool {
    path.iter()
        .filter_map(|component| component.to_str())
        .any(|component| ignored_dir_names.contains(component))
}

/// Build a conventional project scope from a project root.
#[must_use]
pub fn rust_project_harness_scope(
    project_root: &Path,
    include_tests: bool,
    source_dir_names: &[String],
    test_dir_names: &[String],
) -> RustProjectHarnessScope {
    let package_paths = ["build.rs", "examples", "benches"]
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let source_paths = source_dir_names
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let test_paths = if include_tests {
        test_dir_names
            .iter()
            .map(|name| project_root.join(name))
            .filter(|path| path.exists())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    RustProjectHarnessScope {
        project_root: project_root.to_path_buf(),
        source_paths,
        test_paths,
        package_paths,
        fallback_paths: vec![project_root.to_path_buf()],
    }
}
