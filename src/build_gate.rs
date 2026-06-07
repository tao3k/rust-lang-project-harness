//! Build-script entrypoints for `cargo check` project harness gates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::discovery::{discover_rust_files, rust_project_harness_scope};
use crate::model::{RustHarnessConfig, RustHarnessReport};
use crate::runner::run_rust_project_harness_with_config;
use crate::verification::{RustVerificationTaskKind, plan_rust_project_verification_with_config};

/// Assert a project harness run from a Cargo build script.
///
/// This runs during Cargo build-script execution, so `cargo check` surfaces the
/// parser-native harness policy before Cargo reaches the test/evaluation layer.
/// By default, non-blocking agent advice is also treated as repair feedback.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_build_clean(project_root: &Path) -> RustHarnessReport {
    let config = RustHarnessConfig::default();
    emit_cargo_rerun_inputs(project_root, &config);
    let report = run_rust_project_harness_with_config(project_root, &config)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_build_report_clean(&report, &config);
    report
}

/// Assert a configured project harness run from a Cargo build script.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    emit_cargo_rerun_inputs(project_root, config);
    let report = run_rust_project_harness_with_config(project_root, config)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_build_report_clean(&report, config);
    report
}

/// Assert a project harness run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
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
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_build_clean_from_env_with_config(
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_build_clean_with_config(&root, config)
}

/// Assert a project harness policy run from a Cargo build script during `cargo check`.
///
/// This is the preferred downstream entrypoint. The older
/// `assert_rust_project_harness_build_clean(...)` name remains as a
/// compatibility alias.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean(project_root: &Path) -> RustHarnessReport {
    assert_rust_project_harness_build_clean(project_root)
}

/// Assert a configured cargo-check project harness policy run.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist without a cargo-check explanation.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    assert_rust_project_harness_build_clean_with_config(project_root, config)
}

/// Assert a cargo-check project harness policy run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_from_env() -> RustHarnessReport {
    assert_rust_project_harness_build_clean_from_env()
}

/// Assert a configured cargo-check project harness policy run from `CARGO_MANIFEST_DIR`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the run fails, or when
/// configured-blocking findings or unexplained advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_check_clean_from_env_with_config(
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    assert_rust_project_harness_build_clean_from_env_with_config(config)
}

/// Assert that a cargo-check build gate has an active performance verification task.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, the verification plan cannot be
/// built, or the configured plan lacks an active performance task/report.
#[track_caller]
pub fn assert_rust_project_harness_performance_verification_from_env(
    config: &RustHarnessConfig,
    gate_label: &str,
) {
    let root = cargo_manifest_dir();
    let plan = plan_rust_project_verification_with_config(&root, config)
        .unwrap_or_else(|error| panic!("{gate_label} verification plan: {error}"));
    assert!(
        plan.tasks
            .iter()
            .any(|task| task.kind == RustVerificationTaskKind::Performance && task.is_active()),
        "{gate_label} build gate must configure active performance verification tasks"
    );
    assert!(
        plan.report_obligations
            .iter()
            .any(|obligation| obligation.key == "performance_index_json"),
        "{gate_label} build gate must require a performance index report"
    );
}

fn cargo_manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is not set"))
}

fn assert_build_report_clean(report: &RustHarnessReport, config: &RustHarnessConfig) {
    report.assert_clean();
    if !config_allows_agent_advice(config) {
        report.assert_no_advisory_findings();
    }
}

fn config_allows_agent_advice(config: &RustHarnessConfig) -> bool {
    has_explanation(config.cargo_check_advice_allow_explanation.as_deref())
        || has_explanation(config.agent_advice_allow_explanation.as_deref())
}

fn has_explanation(explanation: Option<&str>) -> bool {
    explanation.is_some_and(|explanation| !explanation.trim().is_empty())
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
    for path in discover_rust_files(
        &[project_root.to_path_buf()],
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    ) {
        paths.insert(path);
    }
    paths.retain(|path| path.exists());
    paths
}
