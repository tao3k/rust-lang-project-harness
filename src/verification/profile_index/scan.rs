//! Project scanner that builds verification profile candidates from parser facts.

use std::path::{Path, PathBuf};

use crate::RustProjectHarnessScope;
use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::model::RustHarnessConfig;
use crate::parser::{
    ParsedRustModule, parse_cargo_dependency_facts, parse_rust_file, rust_reasoning_tree_facts,
};
use crate::verification::RustVerificationPolicy;

use super::collect::{PackageCandidateInput, collect_package_candidates};
use super::model::RustVerificationProfileIndex;

/// Build parser-suggested responsibility profile candidates for a Rust project.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index(
    project_root: &Path,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_config(project_root, &RustHarnessConfig::default())
}

/// Build parser-suggested responsibility profile candidates with config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_policy(
        project_root,
        config,
        &config.verification_policy,
    )
}

/// Build parser-suggested responsibility profile candidates with policy.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_policy(
    project_root: &Path,
    config: &RustHarnessConfig,
    policy: &RustVerificationPolicy,
) -> Result<RustVerificationProfileIndex, String> {
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    let package_roots = discover_cargo_package_roots(project_root, &config.ignored_dir_names);
    let package_roots = if should_run_member_scopes(project_root, &package_roots) {
        package_roots
    } else {
        vec![project_root.to_path_buf()]
    };
    let mut candidates = Vec::new();
    for package_root in package_roots {
        let cargo_dependencies = parse_cargo_dependency_facts(&package_root);
        let scope = rust_project_harness_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let parsed_modules = parse_scope(&scope, config);
        let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
        collect_package_candidates(
            PackageCandidateInput {
                project_root,
                package_root: &reasoning_tree.package_root,
                modules: &reasoning_tree.modules,
                branches: &reasoning_tree.owner_branches,
                parsed_modules: &parsed_modules,
                cargo_dependencies: &cargo_dependencies,
                policy,
            },
            &mut candidates,
        );
    }
    candidates.sort_by(|left, right| {
        left.package_root
            .cmp(&right.package_root)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
    });
    Ok(RustVerificationProfileIndex {
        project_root: project_root.to_path_buf(),
        candidates,
        configured_profile_hint_count: policy.profile_hints.len(),
    })
}

fn parse_scope(
    scope: &RustProjectHarnessScope,
    config: &RustHarnessConfig,
) -> Vec<ParsedRustModule> {
    discover_rust_files(&scope.monitored_paths(), &config.ignored_dir_names)
        .into_iter()
        .map(|path| parse_rust_file(&path))
        .collect()
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}
