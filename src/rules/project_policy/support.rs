//! Shared project-policy helpers.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::rules::display_path;

pub(super) fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

pub(super) fn resolve_path_attr(source_file: &Path, path_value: &str) -> PathBuf {
    normalize_path(
        source_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path_value),
    )
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(_) | Component::Prefix(_) | Component::RootDir => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

pub(super) fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if is_rust_file(&path) {
            files.push(path);
        }
    }
}

pub(super) fn display_project_path(project_root: &Path, path: &Path) -> String {
    display_path(path.strip_prefix(project_root).unwrap_or(path))
}
