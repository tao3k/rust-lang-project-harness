//! Cargo manifest facts owned by the parser layer.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const HARNESS_PACKAGE_NAME: &str = "rust-lang-project-harness";

#[derive(Debug, Clone, Default)]
pub(crate) struct CargoManifestFacts {
    pub(crate) has_package: bool,
    pub(crate) workspace_members: Vec<String>,
    pub(crate) workspace_excludes: Vec<String>,
    pub(crate) test_target_files: Vec<PathBuf>,
    pub(crate) references_harness: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct CargoManifestToml {
    package: Option<CargoPackageToml>,
    workspace: Option<CargoWorkspaceToml>,
    test: Vec<CargoTestTargetToml>,
    dependencies: BTreeMap<String, toml::Value>,
    dev_dependencies: BTreeMap<String, toml::Value>,
    build_dependencies: BTreeMap<String, toml::Value>,
    target: BTreeMap<String, CargoTargetManifestToml>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CargoPackageToml {}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CargoWorkspaceToml {
    members: Vec<String>,
    exclude: Vec<String>,
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

pub(crate) fn parse_cargo_manifest(project_root: &Path) -> CargoManifestFacts {
    let Some(manifest) = read_manifest(project_root) else {
        return CargoManifestFacts::default();
    };
    let references_harness = manifest_references_harness(&manifest);
    let has_package = manifest.package.is_some();
    let workspace = manifest.workspace.unwrap_or_default();
    let test_target_files = manifest_test_target_files(project_root, manifest.test);
    CargoManifestFacts {
        has_package,
        workspace_members: workspace.members,
        workspace_excludes: workspace.exclude,
        test_target_files,
        references_harness,
    }
}

fn read_manifest(project_root: &Path) -> Option<CargoManifestToml> {
    let content = fs::read_to_string(project_root.join("Cargo.toml")).ok()?;
    toml::from_str::<CargoManifestToml>(&content).ok()
}

fn manifest_test_target_files(
    project_root: &Path,
    test_targets: Vec<CargoTestTargetToml>,
) -> Vec<PathBuf> {
    test_targets
        .into_iter()
        .filter_map(|target| {
            let target_path = target.path.trim();
            (!target_path.is_empty()).then(|| project_root.join(target_path))
        })
        .collect()
}

fn manifest_references_harness(manifest: &CargoManifestToml) -> bool {
    dependency_table_references_harness(&manifest.dependencies)
        || dependency_table_references_harness(&manifest.dev_dependencies)
        || dependency_table_references_harness(&manifest.build_dependencies)
        || manifest.target.values().any(|target| {
            dependency_table_references_harness(&target.dependencies)
                || dependency_table_references_harness(&target.dev_dependencies)
                || dependency_table_references_harness(&target.build_dependencies)
        })
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
