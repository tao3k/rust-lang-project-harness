//! Cargo test target discovery and parsing.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{CargoManifestFacts, ParsedRustModule, parse_rust_file};

pub(crate) fn parse_cargo_test_targets(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
) -> Vec<ParsedRustModule> {
    collect_test_target_files(project_root, cargo_manifest)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn collect_test_target_files(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
) -> Vec<PathBuf> {
    let mut targets = BTreeSet::new();
    let tests_dir = project_root.join("tests");
    if let Ok(entries) = fs::read_dir(&tests_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_rust_file(&path) {
                targets.insert(path);
            }
        }
    }
    targets.extend(cargo_manifest.test_target_files.iter().cloned());
    targets.into_iter().collect()
}

fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}
