//! File discovery for conventional Rust project layouts.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::RustProjectHarnessScope;

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
