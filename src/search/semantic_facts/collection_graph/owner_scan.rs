//! Candidate owner discovery for collection semantic facts.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const CANDIDATE_OWNER_LIMIT: usize = 16;
const PROJECT_SCAN_OWNER_LIMIT: usize = 256;
const PROJECT_SCAN_DIRECTORY_LIMIT: usize = 2048;

pub(in crate::search::semantic_facts) struct CandidateOwner {
    pub(super) display: String,
    pub(super) absolute: PathBuf,
}

fn candidate_owners(project_root: &Path, input: &str) -> Vec<CandidateOwner> {
    let mut seen = HashSet::new();
    input
        .lines()
        .filter_map(|line| line.split_once(':').map(|(path, _)| path))
        .filter(|path| !path.trim().is_empty())
        .filter_map(|path| {
            let display = owner_display_path(path);
            let absolute = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                project_root.join(path)
            };
            (absolute.exists() && seen.insert(display.clone()))
                .then_some(CandidateOwner { display, absolute })
        })
        .collect()
}

pub(in crate::search::semantic_facts) fn semantic_fact_owners(
    project_root: &Path,
    input: &str,
) -> Vec<CandidateOwner> {
    let mut seen = HashSet::new();
    candidate_owners(project_root, input)
        .into_iter()
        .take(CANDIDATE_OWNER_LIMIT)
        .chain(project_collection_field_owners(project_root))
        .filter(|owner| seen.insert(owner.display.clone()))
        .collect()
}

fn project_collection_field_owners(project_root: &Path) -> Vec<CandidateOwner> {
    let mut files = project_scan_rust_files(project_root);
    files.sort_by_key(|path| (project_scan_file_priority(project_root, path), path.clone()));
    files
        .into_iter()
        .take(PROJECT_SCAN_OWNER_LIMIT)
        .filter_map(|absolute| {
            let display = absolute.strip_prefix(project_root).ok()?.to_string_lossy();
            Some(CandidateOwner {
                display: owner_display_path(&display),
                absolute,
            })
        })
        .collect()
}

fn owner_display_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn project_scan_rust_files(project_root: &Path) -> Vec<PathBuf> {
    let mut directories = vec![project_root.to_path_buf()];
    let mut visited_directories = 0usize;
    let mut files = Vec::new();
    while let Some(directory) =
        next_project_scan_directory(&mut directories, &mut visited_directories)
    {
        let Some(paths) = sorted_directory_paths(&directory) else {
            continue;
        };
        route_project_scan_paths(paths, &mut directories, &mut files);
    }
    files
}

fn next_project_scan_directory(
    directories: &mut Vec<PathBuf>,
    visited_directories: &mut usize,
) -> Option<PathBuf> {
    if *visited_directories >= PROJECT_SCAN_DIRECTORY_LIMIT {
        return None;
    }
    let directory = directories.pop()?;
    *visited_directories += 1;
    Some(directory)
}

fn sorted_directory_paths(directory: &Path) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(directory).ok()?;
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    Some(paths)
}

fn route_project_scan_paths(
    paths: Vec<PathBuf>,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    for path in paths.into_iter().rev() {
        route_project_scan_path(path, directories, files);
    }
}

fn route_project_scan_path(
    path: PathBuf,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    if path.is_dir() {
        if !is_skipped_project_scan_directory(&path) {
            directories.push(path);
        }
    } else if path.extension().is_some_and(|extension| extension == "rs") {
        files.push(path);
    }
}

fn is_skipped_project_scan_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".cache" | "target" | "node_modules" | "vendor"
            )
        })
}

fn project_scan_file_priority(project_root: &Path, path: &Path) -> u8 {
    let relative = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy();
    let normalized = relative.replace('\\', "/");
    let is_test_like = normalized.contains("/tests/")
        || normalized.starts_with("tests/")
        || normalized.contains("/examples/")
        || normalized.starts_with("examples/")
        || normalized.contains("/benches/")
        || normalized.starts_with("benches/")
        || normalized.contains("stress-test/");
    if normalized.contains("/src/") && !is_test_like {
        0
    } else if normalized.contains("/src/") {
        1
    } else if !is_test_like {
        2
    } else {
        3
    }
}
