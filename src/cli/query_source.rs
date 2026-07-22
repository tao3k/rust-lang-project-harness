use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum QuerySourceVersion {
    #[default]
    Worktree,
}

pub(super) fn query_source_path(project_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Some(stripped) = workspace_package_prefixed_path(project_root, path) {
        return project_root.join(stripped);
    }
    project_root.join(path)
}

pub(super) fn read_query_source_text(
    _project_root: &Path,
    _selector_path: &str,
    source_path: &Path,
    _source_version: QuerySourceVersion,
) -> Result<String, String> {
    fs::read_to_string(source_path)
        .map_err(|error| format!("failed to read {}: {error}", source_path.display()))
}

fn workspace_package_prefixed_path(project_root: &Path, path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    let Some(Component::Normal(prefix)) = components.next() else {
        return None;
    };
    if prefix != "languages" {
        return None;
    }
    let Some(Component::Normal(package)) = components.next() else {
        return None;
    };
    if project_root.file_name()? != package {
        return None;
    }
    let stripped = components.collect::<PathBuf>();
    (!stripped.as_os_str().is_empty()).then_some(stripped)
}
