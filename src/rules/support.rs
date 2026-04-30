//! Shared rule-pack support helpers.

use std::path::{Path, PathBuf};

pub(crate) fn labels(
    domain: &'static str,
) -> std::collections::BTreeMap<&'static str, &'static str> {
    std::collections::BTreeMap::from([("language", "rust"), ("domain", domain)])
}

pub(crate) fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}

pub(crate) fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
