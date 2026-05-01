//! Rust path-attribute and literal include path resolution.

use std::path::{Component, Path, PathBuf};

pub(crate) fn resolve_rust_path_attr(source_file: &Path, path_value: &str) -> PathBuf {
    normalize_rust_path(
        source_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path_value),
    )
}

pub(crate) fn resolve_rust_include_literal(source_file: &Path, include_target: &str) -> PathBuf {
    normalize_rust_path(
        source_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(include_target),
    )
}

fn normalize_rust_path(path: PathBuf) -> PathBuf {
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
