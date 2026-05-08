//! Build-script entrypoints for filter-proof project harness gates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::model::{RustHarnessConfig, RustHarnessReport};
use crate::runner::{run_rust_project_harness, run_rust_project_harness_with_config};

/// Assert a project harness run from a Cargo build script.
///
/// Unlike `#[test]`-backed gates, this runs during Cargo build-script
/// execution, before libtest applies any test-name filter.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean(project_root: &Path) -> RustHarnessReport {
    emit_cargo_rerun_inputs(project_root, &RustHarnessConfig::default());
    let report = run_rust_project_harness(project_root).unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

/// Assert a configured project harness run from a Cargo build script.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    emit_cargo_rerun_inputs(project_root, config);
    let report = run_rust_project_harness_with_config(project_root, config)
        .unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

/// Assert a project harness run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_from_env() -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_build_clean(&root)
}

/// Assert a configured project harness run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_from_env_with_config(
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_build_clean_with_config(&root, config)
}

fn cargo_manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is not set"))
}

fn emit_cargo_rerun_inputs(project_root: &Path, config: &RustHarnessConfig) {
    for path in rerun_inputs(project_root, config) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn rerun_inputs(project_root: &Path, config: &RustHarnessConfig) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::from([
        project_root.join("Cargo.toml"),
        project_root.join("Cargo.lock"),
        project_root.join("build.rs"),
    ]);
    let scope = rust_project_harness_scope(
        project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    for path in scope.monitored_paths() {
        paths.insert(path);
    }
    for path in discover_rust_files(&[project_root.to_path_buf()], &config.ignored_dir_names) {
        paths.insert(path);
    }
    paths.retain(|path| path.exists());
    paths
}
