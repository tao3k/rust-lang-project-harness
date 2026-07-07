//! Shared project-policy helpers.

use std::path::Path;

use crate::rules::display_path;

pub(super) fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

pub(super) fn display_project_path(project_root: &Path, path: &Path) -> String {
    display_path(path.strip_prefix(project_root).unwrap_or(path))
}
