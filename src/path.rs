use std::path::{Component, Path, PathBuf};

pub(crate) fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

pub(crate) fn display_project_path(project_root: &Path, path: &Path) -> String {
    let normalized_root = normalize_lexical_path(project_root);
    let normalized_path = normalize_lexical_path(path);
    normalized_path
        .strip_prefix(&normalized_root)
        .unwrap_or(&normalized_path)
        .display()
        .to_string()
        .replace('\\', "/")
}
