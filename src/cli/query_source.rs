use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum QuerySourceVersion {
    #[default]
    Worktree,
    Index,
    Head,
}

impl QuerySourceVersion {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Worktree => "worktree",
            Self::Index => "index",
            Self::Head => "head",
        }
    }

    fn git_object_prefix(self) -> Option<&'static str> {
        match self {
            Self::Worktree => None,
            Self::Index => Some(":"),
            Self::Head => Some("HEAD:"),
        }
    }
}

pub(super) fn parse_query_source_version(value: &str) -> Result<QuerySourceVersion, String> {
    match value {
        "worktree" => Ok(QuerySourceVersion::Worktree),
        "index" => Ok(QuerySourceVersion::Index),
        "head" => Ok(QuerySourceVersion::Head),
        _ => Err(format!(
            "unknown query --source value: {value}; expected worktree, index, or head"
        )),
    }
}

pub(super) struct QueryGitSourceMetadata {
    pub(super) repository_root: PathBuf,
    pub(super) git_blob_oid: String,
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
    project_root: &Path,
    selector_path: &str,
    source_path: &Path,
    source_version: QuerySourceVersion,
) -> Result<String, String> {
    if source_version == QuerySourceVersion::Worktree {
        return fs::read_to_string(source_path)
            .map_err(|error| format!("failed to read {}: {error}", source_path.display()));
    }
    let (repository_root, repo_relative_path) =
        git_source_locator(project_root, selector_path, source_path)?;
    let object = git_object_name(source_version, &repo_relative_path)?;
    git_output(&repository_root, &["show", &object]).and_then(|output| {
        String::from_utf8(output).map_err(|error| {
            format!(
                "failed to decode {} source {} as UTF-8: {error}",
                source_version.as_str(),
                repo_relative_path
            )
        })
    })
}

pub(super) fn git_source_metadata(
    project_root: &Path,
    selector_path: &str,
    source_path: &Path,
    source_version: QuerySourceVersion,
) -> Result<Option<QueryGitSourceMetadata>, String> {
    if source_version == QuerySourceVersion::Worktree {
        return Ok(None);
    }
    let (repository_root, repo_relative_path) =
        git_source_locator(project_root, selector_path, source_path)?;
    let object = git_object_name(source_version, &repo_relative_path)?;
    let git_blob_oid = String::from_utf8(git_output(&repository_root, &["rev-parse", &object])?)
        .map_err(|error| {
            format!(
                "failed to decode {} blob oid for {} as UTF-8: {error}",
                source_version.as_str(),
                repo_relative_path
            )
        })?
        .trim()
        .to_string();
    Ok(Some(QueryGitSourceMetadata {
        repository_root,
        git_blob_oid,
    }))
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

fn git_source_locator(
    project_root: &Path,
    selector_path: &str,
    source_path: &Path,
) -> Result<(PathBuf, String), String> {
    let repository_root = git_repository_root(project_root)?;
    let repository_root = fs::canonicalize(&repository_root).unwrap_or(repository_root);
    let absolute_source_path = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        query_source_path(project_root, selector_path)
    };
    let absolute_source_path =
        fs::canonicalize(&absolute_source_path).unwrap_or(absolute_source_path);
    let repo_relative_path = absolute_source_path
        .strip_prefix(&repository_root)
        .map_err(|_| {
            format!(
                "query --source git reads require {} to be under repository root {}",
                absolute_source_path.display(),
                repository_root.display()
            )
        })?;
    Ok((repository_root, path_to_git_path(repo_relative_path)?))
}

fn git_repository_root(project_root: &Path) -> Result<PathBuf, String> {
    let output = git_output(project_root, &["rev-parse", "--show-toplevel"])?;
    let root = String::from_utf8(output)
        .map_err(|error| format!("failed to decode git repository root as UTF-8: {error}"))?;
    Ok(PathBuf::from(root.trim()))
}

fn git_object_name(
    source_version: QuerySourceVersion,
    repo_relative_path: &str,
) -> Result<String, String> {
    let Some(prefix) = source_version.git_object_prefix() else {
        return Err("worktree source has no git object name".to_string());
    };
    Ok(format!("{prefix}{repo_relative_path}"))
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "git {} failed in {}: {}",
        args.join(" "),
        cwd.display(),
        stderr.trim()
    ))
}

fn path_to_git_path(path: &Path) -> Result<String, String> {
    let git_path = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if git_path.is_empty() {
        Err("query --source git read resolved to repository root".to_string())
    } else {
        Ok(git_path)
    }
}
