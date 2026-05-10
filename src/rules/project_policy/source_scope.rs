//! Harness scope configuration policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::file_location;
use crate::{RustHarnessConfig, RustHarnessFinding, RustHarnessRule};

use super::RUST_PROJ_R013;
use super::support::display_project_path;

const DEFAULT_SOURCE_PATHS: &[&str] = &["src"];
const DEFAULT_TEST_PATHS: &[&str] = &["tests"];

pub(super) fn source_scope_findings(
    project_root: &Path,
    config: &RustHarnessConfig,
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let rule = &rules[RUST_PROJ_R013];
    config
        .source_dir_names
        .iter()
        .filter(|path| {
            custom_path_lacks_explanation(
                path,
                DEFAULT_SOURCE_PATHS,
                &config.source_path_explanations,
            )
        })
        .map(|path| scope_path_finding(project_root, path, "source", rule))
        .chain(
            config
                .test_dir_names
                .iter()
                .filter(|path| {
                    custom_path_lacks_explanation(
                        path,
                        DEFAULT_TEST_PATHS,
                        &config.test_path_explanations,
                    )
                })
                .map(|path| scope_path_finding(project_root, path, "test", rule)),
        )
        .collect()
}

fn custom_path_lacks_explanation(
    path: &str,
    default_paths: &[&str],
    explanations: &BTreeMap<String, String>,
) -> bool {
    let normalized = normalize_scope_path(path);
    !normalized.is_empty()
        && !default_paths.contains(&normalized.as_str())
        && explanations
            .get(&normalized)
            .is_none_or(|explanation| explanation.trim().is_empty())
}

fn scope_path_finding(
    project_root: &Path,
    path: &str,
    scope_kind: &str,
    rule: &RustHarnessRule,
) -> RustHarnessFinding {
    let normalized = normalize_scope_path(path);
    let absolute_path = project_root.join(&normalized);
    RustHarnessFinding::from_rule(
        rule,
        format!(
            "Config adds custom {scope_kind} scope path `{}` without a non-empty explanation.",
            display_project_path(project_root, &absolute_path)
        ),
        file_location(if absolute_path.exists() {
            absolute_path
        } else {
            project_root.to_path_buf()
        }),
        None,
        "use with_source_path(path, explanation) or with_test_path(path, explanation) so the Agent explains why this scope is necessary",
    )
}

fn normalize_scope_path(path: &str) -> String {
    path.trim().trim_matches('/').replace('\\', "/")
}
