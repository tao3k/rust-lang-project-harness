//! Project-policy layout configuration.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use serde::Deserialize;

const RULE_CONFIG_FILE: &str = "tests/rust-project-harness-rules.toml";

const ALLOWED_TEST_DIRS: &[&str] = &[
    "common",
    "fixtures",
    "integration",
    "performance",
    "scenarios",
    "snapshots",
    "support",
    "unit",
];

const ALLOWED_TEST_ROOT_FILES: &[&str] = &[
    "integration_test.rs",
    "lib.rs",
    "mod.rs",
    "performance_test.rs",
    "rust-project-harness-gate.rs",
    "scenarios_test.rs",
    "unit_test.rs",
    "xiuxian-testing-gate.rs",
];

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RuleConfigToml {
    tests: TestsToml,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TestsToml {
    allowed_root_files: Vec<AllowedEntryToml>,
    allowed_directories: Vec<AllowedEntryToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AllowedEntryToml {
    name: String,
    explanation: String,
}

#[derive(Debug, Default)]
pub(super) struct LayoutPolicy {
    allowed_root_files: BTreeSet<String>,
    allowed_directories: BTreeSet<String>,
}

pub(super) fn load_layout_policy(project_root: &Path) -> LayoutPolicy {
    let config_path = project_root.join(RULE_CONFIG_FILE);
    let Ok(content) = fs::read_to_string(&config_path) else {
        return LayoutPolicy::default();
    };
    let Ok(parsed) = toml::from_str::<RuleConfigToml>(&content) else {
        return LayoutPolicy::default();
    };
    LayoutPolicy {
        allowed_root_files: parsed
            .tests
            .allowed_root_files
            .into_iter()
            .filter(|entry| !entry.name.trim().is_empty() && !entry.explanation.trim().is_empty())
            .map(|entry| entry.name)
            .collect(),
        allowed_directories: parsed
            .tests
            .allowed_directories
            .into_iter()
            .filter(|entry| !entry.name.trim().is_empty() && !entry.explanation.trim().is_empty())
            .map(|entry| entry.name)
            .collect(),
    }
}

pub(super) fn is_allowed_test_dir(name: &str, policy: &LayoutPolicy) -> bool {
    ALLOWED_TEST_DIRS.contains(&name) || policy.allowed_directories.contains(name)
}

pub(super) fn is_allowed_test_root_file(name: &str, policy: &LayoutPolicy) -> bool {
    ALLOWED_TEST_ROOT_FILES.contains(&name) || policy.allowed_root_files.contains(name)
}

pub(super) fn is_allowed_test_suite_path(path: &Path, policy: &LayoutPolicy) -> bool {
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    if first != "tests" {
        return false;
    }
    let Some(Component::Normal(suite)) = components.next() else {
        return false;
    };
    suite
        .to_str()
        .is_some_and(|name| is_allowed_test_dir(name, policy))
        && path.extension().is_some_and(|extension| extension == "rs")
}
