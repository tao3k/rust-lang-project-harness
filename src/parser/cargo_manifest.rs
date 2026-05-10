//! Cargo manifest facts owned by the parser layer.

use std::fs;
use std::path::{Path, PathBuf};

use cargo_toml::{Dependency, DepsSet, Manifest, Product};

const HARNESS_PACKAGE_NAME: &str = "rust-lang-project-harness";

#[derive(Debug, Clone, Default)]
pub(crate) struct CargoManifestFacts {
    pub(crate) has_package: bool,
    pub(crate) workspace_members: Vec<String>,
    pub(crate) workspace_excludes: Vec<String>,
    pub(crate) source_target_files: Vec<PathBuf>,
    pub(crate) example_target_files: Vec<PathBuf>,
    pub(crate) test_target_files: Vec<PathBuf>,
    pub(crate) bench_targets: Vec<CargoBenchTargetFacts>,
    pub(crate) references_harness: bool,
    pub(crate) references_harness_build_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoBenchTargetFacts {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
    pub(crate) required_features: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CargoDependencyKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoDependencyFacts {
    pub(crate) dependency_key: String,
    pub(crate) import_name: String,
    pub(crate) package_name: String,
    pub(crate) kind: CargoDependencyKind,
    pub(crate) target: Option<String>,
    pub(crate) optional: bool,
    pub(crate) features: Vec<String>,
}

pub(crate) fn parse_cargo_manifest(project_root: &Path) -> CargoManifestFacts {
    let Some(manifest) = read_manifest(project_root) else {
        return CargoManifestFacts::default();
    };
    let references_harness = manifest_references_harness(&manifest);
    let references_harness_build_dependency =
        manifest_references_harness_build_dependency(&manifest);
    let has_package = manifest.package.is_some();
    let (workspace_members, workspace_excludes) = manifest
        .workspace
        .as_ref()
        .map(|workspace| (workspace.members.clone(), workspace.exclude.clone()))
        .unwrap_or_default();
    let source_target_files = manifest_source_target_files(project_root, &manifest);
    let example_target_files = manifest_product_target_files(project_root, &manifest.example);
    let test_target_files = manifest_test_target_files(project_root, &manifest.test);
    let bench_targets = manifest_bench_targets(project_root, &manifest.bench);
    CargoManifestFacts {
        has_package,
        workspace_members,
        workspace_excludes,
        source_target_files,
        example_target_files,
        test_target_files,
        bench_targets,
        references_harness,
        references_harness_build_dependency,
    }
}

pub(crate) fn parse_cargo_dependency_facts(project_root: &Path) -> Vec<CargoDependencyFacts> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    let mut dependencies = manifest_dependency_facts(&manifest);
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

fn read_manifest(project_root: &Path) -> Option<Manifest> {
    let manifest_path = project_root.join("Cargo.toml");
    Manifest::from_path(&manifest_path)
        .or_else(|_| read_manifest_slice(&manifest_path))
        .ok()
}

fn read_manifest_slice(manifest_path: &Path) -> Result<Manifest, cargo_toml::Error> {
    let content = fs::read(manifest_path)?;
    Manifest::from_slice(&content)
}

fn manifest_source_target_files(project_root: &Path, manifest: &Manifest) -> Vec<PathBuf> {
    let mut target_files = Vec::new();
    if let Some(library_target) = &manifest.lib {
        target_files.extend(manifest_product_target_files(
            project_root,
            std::slice::from_ref(library_target),
        ));
    }
    target_files.extend(manifest_product_target_files(project_root, &manifest.bin));
    target_files
}

fn manifest_test_target_files(project_root: &Path, test_targets: &[Product]) -> Vec<PathBuf> {
    manifest_product_target_files(project_root, test_targets)
}

fn manifest_product_target_files(project_root: &Path, targets: &[Product]) -> Vec<PathBuf> {
    targets
        .iter()
        .filter_map(|target| {
            let target_path = target.path.as_deref().unwrap_or_default().trim();
            (!target_path.is_empty()).then(|| project_root.join(target_path))
        })
        .collect()
}

fn manifest_bench_targets(
    project_root: &Path,
    bench_targets: &[Product],
) -> Vec<CargoBenchTargetFacts> {
    bench_targets
        .iter()
        .filter_map(|target| {
            let name = target.name.as_deref()?.trim();
            if name.is_empty() {
                return None;
            }
            let path = target
                .path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map_or_else(
                    || project_root.join("benches").join(format!("{name}.rs")),
                    |path| project_root.join(path),
                );
            let mut required_features = target.required_features.clone();
            required_features.sort();
            required_features.dedup();
            Some(CargoBenchTargetFacts {
                name: name.to_string(),
                path,
                harness: target.harness,
                required_features,
            })
        })
        .collect()
}

fn manifest_references_harness(manifest: &Manifest) -> bool {
    dependency_table_references_harness(&manifest.dependencies)
        || dependency_table_references_harness(&manifest.dev_dependencies)
        || dependency_table_references_harness(&manifest.build_dependencies)
        || manifest.target.values().any(|target| {
            dependency_table_references_harness(&target.dependencies)
                || dependency_table_references_harness(&target.dev_dependencies)
                || dependency_table_references_harness(&target.build_dependencies)
        })
}

fn manifest_references_harness_build_dependency(manifest: &Manifest) -> bool {
    dependency_table_references_harness(&manifest.build_dependencies)
        || manifest
            .target
            .values()
            .any(|target| dependency_table_references_harness(&target.build_dependencies))
}

fn manifest_dependency_facts(manifest: &Manifest) -> Vec<CargoDependencyFacts> {
    let mut dependencies = Vec::new();
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Normal,
        None,
        &manifest.dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Dev,
        None,
        &manifest.dev_dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Build,
        None,
        &manifest.build_dependencies,
    ));
    for (target_name, target) in &manifest.target {
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Normal,
            Some(target_name),
            &target.dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Dev,
            Some(target_name),
            &target.dev_dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Build,
            Some(target_name),
            &target.build_dependencies,
        ));
    }
    dependencies
}

fn dependency_table_facts(
    kind: CargoDependencyKind,
    target: Option<&str>,
    dependencies: &DepsSet,
) -> Vec<CargoDependencyFacts> {
    dependencies
        .iter()
        .map(|(name, dependency)| dependency_fact(name, dependency, kind, target))
        .collect()
}

fn dependency_fact(
    name: &str,
    dependency: &Dependency,
    kind: CargoDependencyKind,
    target: Option<&str>,
) -> CargoDependencyFacts {
    let mut features = dependency.req_features().to_vec();
    features.sort();
    features.dedup();
    CargoDependencyFacts {
        dependency_key: name.to_string(),
        import_name: name.replace('-', "_"),
        package_name: dependency.package().unwrap_or(name).to_string(),
        kind,
        target: target.map(ToOwned::to_owned),
        optional: dependency.optional(),
        features,
    }
}

fn dependency_table_references_harness(dependencies: &DepsSet) -> bool {
    dependencies
        .iter()
        .any(|(name, value)| dependency_references_harness(name, value))
}

fn dependency_references_harness(name: &str, value: &Dependency) -> bool {
    dependency_name_is_harness(name) || value.package().is_some_and(dependency_name_is_harness)
}

fn dependency_name_is_harness(name: &str) -> bool {
    name == HARNESS_PACKAGE_NAME
}
