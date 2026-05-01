//! Cargo manifest facts used by project-policy rules.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const HARNESS_PACKAGE_NAME: &str = "rust-lang-project-harness";

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct CargoManifestToml {
    test: Vec<CargoTestTargetToml>,
    dependencies: BTreeMap<String, toml::Value>,
    dev_dependencies: BTreeMap<String, toml::Value>,
    build_dependencies: BTreeMap<String, toml::Value>,
    target: BTreeMap<String, CargoTargetManifestToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CargoTestTargetToml {
    path: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct CargoTargetManifestToml {
    dependencies: BTreeMap<String, toml::Value>,
    dev_dependencies: BTreeMap<String, toml::Value>,
    build_dependencies: BTreeMap<String, toml::Value>,
}

pub(super) fn manifest_test_target_files(project_root: &Path) -> Vec<PathBuf> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    manifest
        .test
        .into_iter()
        .filter_map(|target| {
            let target_path = target.path.trim();
            (!target_path.is_empty()).then(|| project_root.join(target_path))
        })
        .collect()
}

pub(super) fn manifest_references_harness(project_root: &Path) -> bool {
    let Some(manifest) = read_manifest(project_root) else {
        return false;
    };
    dependency_table_references_harness(&manifest.dependencies)
        || dependency_table_references_harness(&manifest.dev_dependencies)
        || dependency_table_references_harness(&manifest.build_dependencies)
        || manifest.target.values().any(|target| {
            dependency_table_references_harness(&target.dependencies)
                || dependency_table_references_harness(&target.dev_dependencies)
                || dependency_table_references_harness(&target.build_dependencies)
        })
}

fn read_manifest(project_root: &Path) -> Option<CargoManifestToml> {
    let content = fs::read_to_string(project_root.join("Cargo.toml")).ok()?;
    toml::from_str::<CargoManifestToml>(&content).ok()
}

fn dependency_table_references_harness(dependencies: &BTreeMap<String, toml::Value>) -> bool {
    dependencies
        .iter()
        .any(|(name, value)| dependency_references_harness(name, value))
}

fn dependency_references_harness(name: &str, value: &toml::Value) -> bool {
    dependency_name_is_harness(name)
        || value
            .as_table()
            .and_then(|table| table.get("package"))
            .and_then(toml::Value::as_str)
            .is_some_and(dependency_name_is_harness)
}

fn dependency_name_is_harness(name: &str) -> bool {
    name == HARNESS_PACKAGE_NAME
}
