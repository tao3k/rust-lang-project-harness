//! Project-path recency helpers for deterministic search ranking.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) fn compare_paths_by_recency(root: &Path, left: &Path, right: &Path) -> Ordering {
    modified_at(root, right)
        .cmp(&modified_at(root, left))
        .then_with(|| left.cmp(right))
}

fn modified_at(root: &Path, path: &Path) -> Option<SystemTime> {
    let absolute = project_path(root, path)?;
    std::fs::metadata(absolute)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn project_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let root = root.canonicalize().ok()?;
    let absolute = absolute.canonicalize().ok()?;
    if absolute.starts_with(&root) {
        Some(absolute)
    } else {
        None
    }
}
