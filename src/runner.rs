//! Runner API for embedding the Rust project harness in tests and tools.

use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::model::{RustHarnessConfig, RustHarnessReport};
use crate::parser::{ParsedRustModule, parse_rust_file};
use crate::rules::evaluate_default_rule_packs;

/// Return the default Rust harness configuration.
#[must_use]
pub fn default_rust_harness_config() -> RustHarnessConfig {
    RustHarnessConfig::default()
}

/// Run the harness over conventional Rust project paths.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn run_rust_project_harness(project_root: &Path) -> Result<RustHarnessReport, String> {
    run_rust_project_harness_with_config(project_root, &RustHarnessConfig::default())
}

/// Run the harness over conventional Rust project paths with explicit config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn run_rust_project_harness_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustHarnessReport, String> {
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    let scope = rust_project_harness_scope(
        project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let monitored_paths = scope.monitored_paths();
    let mut report = run_paths(&monitored_paths, config);
    report.project_scope = Some(scope);
    let parsed_modules = parse_paths(&monitored_paths, config);
    report.findings = evaluate_default_rule_packs(report.project_scope.as_ref(), &parsed_modules);
    report.modules = parsed_modules
        .into_iter()
        .map(|module| module.report)
        .collect();
    Ok(report)
}

/// Run the harness over explicit files or directories.
///
/// # Errors
///
/// Returns an error when any requested root does not exist.
pub fn run_rust_lang_harness(paths: &[PathBuf]) -> Result<RustHarnessReport, String> {
    run_rust_lang_harness_with_config(paths, &RustHarnessConfig::default())
}

/// Run the harness over explicit files or directories with explicit config.
///
/// # Errors
///
/// Returns an error when any requested root does not exist.
pub fn run_rust_lang_harness_with_config(
    paths: &[PathBuf],
    config: &RustHarnessConfig,
) -> Result<RustHarnessReport, String> {
    for path in paths {
        if !path.exists() {
            return Err(format!("harness path does not exist: {}", path.display()));
        }
    }
    Ok(run_paths(paths, config))
}

/// Assert a conventional Rust project harness run is clean.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_clean(project_root: &Path) -> RustHarnessReport {
    let report = run_rust_project_harness(project_root).unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

/// Assert an explicit-path Rust harness run is clean.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_lang_harness_clean(paths: &[PathBuf]) -> RustHarnessReport {
    let report = run_rust_lang_harness(paths).unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

fn run_paths(paths: &[PathBuf], config: &RustHarnessConfig) -> RustHarnessReport {
    let parsed_modules = parse_paths(paths, config);
    let findings = evaluate_default_rule_packs(None, &parsed_modules);
    RustHarnessReport {
        modules: parsed_modules
            .into_iter()
            .map(|module| module.report)
            .collect(),
        findings,
        root_paths: paths.to_vec(),
        blocking_severities: config.blocking_severities.clone(),
        project_scope: None,
    }
}

fn parse_paths(paths: &[PathBuf], config: &RustHarnessConfig) -> Vec<ParsedRustModule> {
    discover_rust_files(paths, &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}
