//! Runner API for embedding the Rust project harness in tests and tools.

use std::path::{Path, PathBuf};

use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::model::{RustHarnessConfig, RustHarnessReport};
use crate::parser::{ParsedRustModule, parse_rust_file};
use crate::rules::evaluate_default_rule_packs_with_config;

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
    let package_roots = discover_cargo_package_roots(project_root, &config.ignored_dir_names);
    if should_run_member_scopes(project_root, &package_roots) {
        return Ok(run_member_scoped_project_harness(
            project_root,
            &package_roots,
            config,
        ));
    }

    Ok(run_single_project_harness(project_root, config))
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
    let findings = evaluate_default_rule_packs_with_config(None, &parsed_modules, config);
    RustHarnessReport {
        modules: parsed_modules
            .into_iter()
            .map(|module| module.report)
            .collect(),
        findings,
        root_paths: paths.to_vec(),
        blocking_severities: config.blocking_severities.clone(),
        project_scope: None,
        workspace_member_scopes: Vec::new(),
    }
}

fn run_single_project_harness(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let scope = rust_project_harness_scope(
        project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let monitored_paths = scope.monitored_paths();
    let parsed_modules = parse_paths(&monitored_paths, config);
    let findings = evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config);
    RustHarnessReport {
        modules: parsed_modules
            .into_iter()
            .map(|module| module.report)
            .collect(),
        findings,
        root_paths: monitored_paths,
        blocking_severities: config.blocking_severities.clone(),
        project_scope: Some(scope),
        workspace_member_scopes: Vec::new(),
    }
}

fn run_member_scoped_project_harness(
    project_root: &Path,
    package_roots: &[PathBuf],
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let mut modules = Vec::new();
    let mut findings = Vec::new();
    let member_scopes = package_roots
        .iter()
        .map(|package_root| {
            rust_project_harness_scope(
                package_root,
                config.include_tests,
                &config.source_dir_names,
                &config.test_dir_names,
            )
        })
        .collect::<Vec<_>>();
    for scope in &member_scopes {
        let monitored_paths = scope.monitored_paths();
        let parsed_modules = parse_paths(&monitored_paths, config);
        findings.extend(evaluate_default_rule_packs_with_config(
            Some(scope),
            &parsed_modules,
            config,
        ));
        modules.extend(
            parsed_modules
                .into_iter()
                .map(|module| module.report)
                .collect::<Vec<_>>(),
        );
    }
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    RustHarnessReport {
        modules,
        findings,
        root_paths: vec![project_root.to_path_buf()],
        blocking_severities: config.blocking_severities.clone(),
        project_scope: Some(rust_project_harness_scope(
            project_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        )),
        workspace_member_scopes: member_scopes,
    }
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}

fn parse_paths(paths: &[PathBuf], config: &RustHarnessConfig) -> Vec<ParsedRustModule> {
    discover_rust_files(paths, &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}
