//! Cargo manifest facts owned by the parser layer.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_toml::{Dependency, DepsSet, Manifest, Product};
#[cfg(any(feature = "search", test))]
use cargo_toml::{Inheritable, LintGroups, Value};

const HARNESS_PACKAGE_NAME: &str = "rust-lang-project-harness";

#[derive(Debug, Clone, Default)]
pub(crate) struct CargoManifestFacts {
    pub(crate) has_package: bool,
    #[cfg(feature = "cli")]
    pub(crate) package_name: Option<String>,
    pub(crate) workspace_members: Vec<String>,
    pub(crate) workspace_excludes: Vec<String>,
    pub(crate) path_dependency_roots: Vec<PathBuf>,
    pub(crate) source_target_files: Vec<PathBuf>,
    pub(crate) example_targets: Vec<CargoExampleTargetFacts>,
    pub(crate) test_target_files: Vec<PathBuf>,
    pub(crate) bench_targets: Vec<CargoBenchTargetFacts>,
    pub(crate) references_harness: bool,
    pub(crate) references_harness_build_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoExampleTargetFacts {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) required_features: Vec<String>,
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
    pub(crate) version_req: Option<String>,
    pub(crate) kind: CargoDependencyKind,
    pub(crate) target: Option<String>,
    pub(crate) optional: bool,
    pub(crate) features: Vec<String>,
}

#[cfg(any(feature = "search", test))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoCfgFacts {
    pub(crate) cfg: String,
    pub(crate) declared_in: String,
    pub(crate) expression: String,
}

pub(crate) fn parse_cargo_manifest(project_root: &Path) -> CargoManifestFacts {
    let Some(manifest) = read_manifest(project_root) else {
        return CargoManifestFacts::default();
    };
    let references_harness = manifest_references_harness(&manifest);
    let references_harness_build_dependency =
        manifest_references_harness_build_dependency(&manifest);
    let package_name = manifest
        .package
        .as_ref()
        .map(|package| package.name.clone());
    let has_package = package_name
        .as_deref()
        .is_some_and(|name| !name.trim().is_empty());
    let (workspace_members, workspace_excludes) = manifest
        .workspace
        .as_ref()
        .map(|workspace| (workspace.members.clone(), workspace.exclude.clone()))
        .unwrap_or_default();
    let source_target_files = manifest_source_target_files(project_root, &manifest);
    let example_targets = manifest_example_targets(project_root, &manifest.example);
    let test_target_files = manifest_test_target_files(project_root, &manifest.test);
    let bench_targets = manifest_bench_targets(project_root, &manifest.bench);
    let path_dependency_roots = manifest_path_dependency_roots(project_root, &manifest);
    CargoManifestFacts {
        has_package,
        #[cfg(feature = "cli")]
        package_name,
        workspace_members,
        workspace_excludes,
        path_dependency_roots,
        source_target_files,
        example_targets,
        test_target_files,
        bench_targets,
        references_harness,
        references_harness_build_dependency,
    }
}

#[cfg(feature = "cli")]
pub(crate) fn parse_cargo_workspace_member_roots(project_root: &Path) -> Vec<PathBuf> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    let Some(workspace) = manifest.workspace.as_ref() else {
        return Vec::new();
    };
    let mut roots = BTreeSet::new();
    for member in &workspace.members {
        expand_workspace_member_pattern(project_root, member, &mut roots);
    }
    roots.retain(|root| {
        root.join("Cargo.toml").is_file()
            && root.strip_prefix(project_root).ok().is_none_or(|relative| {
                let relative = relative.to_string_lossy().replace('\\', "/");
                !workspace
                    .exclude
                    .iter()
                    .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
            })
    });
    roots.into_iter().collect()
}

#[cfg(feature = "cli")]
pub(crate) fn cargo_project_root_for_path(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to resolve Rust project path {}: {error}",
            path.display()
        )
    })?;
    let mut current = if canonical.is_file() {
        canonical
            .parent()
            .ok_or_else(|| format!("failed to resolve parent for {}", canonical.display()))?
            .to_path_buf()
    } else {
        canonical
    };
    loop {
        if current.join("Cargo.toml").is_file() {
            return Ok(cargo_project_root_for_manifest_dir(&current));
        }
        if !current.pop() {
            break;
        }
    }
    Err("failed to find Rust project root: Cargo.toml not found".to_string())
}

#[cfg(feature = "cli")]
fn cargo_project_root_for_manifest_dir(manifest_dir: &Path) -> PathBuf {
    let manifest = read_manifest(manifest_dir);
    if manifest
        .as_ref()
        .is_some_and(|manifest| manifest.workspace.is_some())
    {
        return manifest_dir.to_path_buf();
    }
    if let Some(workspace_root) = manifest
        .as_ref()
        .and_then(|manifest| manifest.package.as_ref())
        .and_then(|package| package.workspace.as_deref())
        .map(|workspace| manifest_dir.join(workspace))
    {
        return fs::canonicalize(&workspace_root).unwrap_or(workspace_root);
    }
    cargo_parent_workspace_root(manifest_dir).unwrap_or_else(|| manifest_dir.to_path_buf())
}

#[cfg(feature = "cli")]
fn cargo_parent_workspace_root(manifest_dir: &Path) -> Option<PathBuf> {
    let mut current = manifest_dir.parent();
    while let Some(candidate) = current {
        if read_manifest(candidate).as_ref().is_some_and(|manifest| {
            workspace_contains_manifest_dir(candidate, manifest, manifest_dir)
        }) {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

#[cfg(feature = "cli")]
fn workspace_contains_manifest_dir(
    workspace_root: &Path,
    manifest: &Manifest,
    manifest_dir: &Path,
) -> bool {
    let Some(workspace) = manifest.workspace.as_ref() else {
        return false;
    };
    let Ok(relative) = manifest_dir.strip_prefix(workspace_root) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    if workspace
        .exclude
        .iter()
        .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
    {
        return false;
    }
    workspace
        .members
        .iter()
        .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
}

#[cfg(feature = "cli")]
fn workspace_member_pattern_matches(pattern: &str, relative: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == relative;
    }
    let pattern_components = pattern.replace('\\', "/");
    let pattern_components = pattern_components.split('/').collect::<Vec<_>>();
    let relative_components = relative.split('/').collect::<Vec<_>>();
    pattern_components.len() == relative_components.len()
        && pattern_components.iter().zip(relative_components).all(
            |(pattern_component, relative_component)| {
                workspace_member_component_matches(pattern_component, relative_component)
            },
        )
}

#[cfg(feature = "cli")]
fn workspace_member_component_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let mut remaining = value;
    let mut parts = pattern.split('*').peekable();
    let Some(first) = parts.next() else {
        return pattern == value;
    };
    if !remaining.starts_with(first) {
        return false;
    }
    remaining = &remaining[first.len()..];
    while let Some(part) = parts.next() {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
        if parts.peek().is_none() && !remaining.is_empty() {
            return false;
        }
    }
    pattern.ends_with('*') || remaining.is_empty()
}

#[cfg(feature = "cli")]
fn expand_workspace_member_pattern(
    project_root: &Path,
    pattern: &str,
    roots: &mut BTreeSet<PathBuf>,
) {
    let normalized = pattern.replace('\\', "/");
    let components = normalized
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    if components.is_empty() {
        return;
    }
    expand_workspace_member_components(project_root, &components, roots);
}

#[cfg(feature = "cli")]
fn expand_workspace_member_components(
    current: &Path,
    components: &[&str],
    roots: &mut BTreeSet<PathBuf>,
) {
    let Some((component, remaining)) = components.split_first() else {
        roots.insert(current.to_path_buf());
        return;
    };
    if !component.contains('*') {
        expand_workspace_member_components(&current.join(component), remaining, roots);
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if workspace_member_component_matches(component, name) {
            expand_workspace_member_components(&path, remaining, roots);
        }
    }
}

fn manifest_example_targets(
    project_root: &Path,
    example_targets: &[Product],
) -> Vec<CargoExampleTargetFacts> {
    example_targets
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
                    || project_root.join("examples").join(format!("{name}.rs")),
                    |path| project_root.join(path),
                );
            let mut required_features = target.required_features.clone();
            required_features.sort();
            required_features.dedup();
            Some(CargoExampleTargetFacts {
                name: name.to_string(),
                path,
                required_features,
            })
        })
        .collect()
}

pub(crate) fn parse_cargo_dependency_facts(project_root: &Path) -> Vec<CargoDependencyFacts> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    let mut dependencies = manifest_dependency_facts(project_root, &manifest);
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

#[cfg(any(feature = "search", test))]
pub(crate) fn parse_cargo_cfg_facts(project_root: &Path) -> Vec<CargoCfgFacts> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    let mut cfgs = manifest_cfg_facts(&manifest);
    cfgs.sort();
    cfgs.dedup();
    cfgs
}

fn manifest_path_dependency_roots(project_root: &Path, manifest: &Manifest) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    collect_path_dependency_roots(project_root, &manifest.dependencies, &mut roots);
    collect_path_dependency_roots(project_root, &manifest.dev_dependencies, &mut roots);
    collect_path_dependency_roots(project_root, &manifest.build_dependencies, &mut roots);
    for target in manifest.target.values() {
        collect_path_dependency_roots(project_root, &target.dependencies, &mut roots);
        collect_path_dependency_roots(project_root, &target.dev_dependencies, &mut roots);
        collect_path_dependency_roots(project_root, &target.build_dependencies, &mut roots);
    }
    roots.into_iter().collect()
}

fn collect_path_dependency_roots(
    project_root: &Path,
    dependencies: &DepsSet,
    roots: &mut BTreeSet<PathBuf>,
) {
    for (name, dependency) in dependencies {
        if let Some(root) = dependency_path_root(project_root, name, dependency) {
            roots.insert(root);
        }
    }
}

fn dependency_path_root(
    project_root: &Path,
    name: &str,
    dependency: &Dependency,
) -> Option<PathBuf> {
    dependency
        .detail()
        .and_then(|detail| detail.path.as_deref())
        .map(|path| resolve_dependency_path(project_root, path))
        .or_else(|| workspace_dependency_path_root(project_root, name))
}

fn workspace_dependency_path_root(project_root: &Path, name: &str) -> Option<PathBuf> {
    project_root.ancestors().skip(1).find_map(|workspace_root| {
        let manifest = read_manifest(workspace_root)?;
        manifest
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.dependencies.get(name))
            .and_then(|dependency| dependency.detail())
            .and_then(|detail| detail.path.as_deref())
            .map(|path| resolve_dependency_path(workspace_root, path))
    })
}

fn resolve_dependency_path(base: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
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

fn manifest_dependency_facts(
    project_root: &Path,
    manifest: &Manifest,
) -> Vec<CargoDependencyFacts> {
    let mut dependencies = Vec::new();
    dependencies.extend(dependency_table_facts(
        project_root,
        CargoDependencyKind::Normal,
        None,
        &manifest.dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        project_root,
        CargoDependencyKind::Dev,
        None,
        &manifest.dev_dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        project_root,
        CargoDependencyKind::Build,
        None,
        &manifest.build_dependencies,
    ));
    for (target_name, target) in &manifest.target {
        dependencies.extend(dependency_table_facts(
            project_root,
            CargoDependencyKind::Normal,
            Some(target_name),
            &target.dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            project_root,
            CargoDependencyKind::Dev,
            Some(target_name),
            &target.dev_dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            project_root,
            CargoDependencyKind::Build,
            Some(target_name),
            &target.build_dependencies,
        ));
    }
    dependencies
}

#[cfg(any(feature = "search", test))]
fn manifest_cfg_facts(manifest: &Manifest) -> Vec<CargoCfgFacts> {
    let mut cfgs = Vec::new();
    cfgs.extend(feature_cfg_facts(&manifest.features));
    cfgs.extend(lint_cfg_facts(
        "workspace.lints.rust.unexpected_cfgs",
        manifest
            .workspace
            .as_ref()
            .map(|workspace| &workspace.lints),
    ));
    if let Ok(lints) = manifest.lints.get() {
        cfgs.extend(lint_cfg_facts("lints.rust.unexpected_cfgs", Some(lints)));
    } else if matches!(manifest.lints, Inheritable::Inherited) {
        cfgs.push(CargoCfgFacts {
            cfg: "workspace".to_string(),
            declared_in: "lints".to_string(),
            expression: "workspace=true".to_string(),
        });
    }
    for target_name in manifest.target.keys() {
        cfgs.extend(target_cfg_facts(target_name));
    }
    cfgs
}

#[cfg(any(feature = "search", test))]
fn feature_cfg_facts(features: &cargo_toml::FeatureSet) -> Vec<CargoCfgFacts> {
    features
        .keys()
        .map(|name| CargoCfgFacts {
            cfg: format!("feature:{name}"),
            declared_in: "features".to_string(),
            expression: format!("cfg(feature=\"{name}\")"),
        })
        .collect()
}

#[cfg(any(feature = "search", test))]
fn lint_cfg_facts(declared_in: &str, lints: Option<&LintGroups>) -> Vec<CargoCfgFacts> {
    let Some(lint) = lints
        .and_then(|groups| groups.get("rust"))
        .and_then(|rust| rust.get("unexpected_cfgs"))
    else {
        return Vec::new();
    };
    lint.config
        .get("check-cfg")
        .into_iter()
        .flat_map(cargo_cfg_strings)
        .flat_map(|expression| cfg_facts_for_expression(declared_in, &expression))
        .collect()
}

#[cfg(any(feature = "search", test))]
fn cargo_cfg_strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(any(feature = "search", test))]
fn target_cfg_facts(target_name: &str) -> Vec<CargoCfgFacts> {
    cfg_facts_for_expression("target.dependencies", target_name)
}

#[cfg(any(feature = "search", test))]
fn cfg_facts_for_expression(declared_in: &str, expression: &str) -> Vec<CargoCfgFacts> {
    let expression = compact_cfg_expression(expression);
    cfg_labels_from_expression(&expression)
        .into_iter()
        .map(|cfg| CargoCfgFacts {
            cfg,
            declared_in: declared_in.to_string(),
            expression: expression.clone(),
        })
        .collect()
}

#[cfg(any(feature = "search", test))]
fn cfg_labels_from_expression(expression: &str) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    let mut token = String::new();
    let mut in_quote = false;
    let has_feature_cfg = expression_has_token(expression, "feature");
    for character in expression.chars() {
        if character == '"' {
            if in_quote && has_feature_cfg && !token.is_empty() {
                labels.insert(format!("feature:{token}"));
            }
            token.clear();
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            token.push(character);
            continue;
        }
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            token.push(character);
            continue;
        }
        push_cfg_label(&mut labels, &mut token);
    }
    push_cfg_label(&mut labels, &mut token);
    labels
}

#[cfg(any(feature = "search", test))]
fn expression_has_token(expression: &str, needle: &str) -> bool {
    let mut token = String::new();
    let mut in_quote = false;
    for character in expression.chars() {
        if character == '"' {
            token.clear();
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            token.push(character);
            continue;
        }
        if token == needle {
            return true;
        }
        token.clear();
    }
    token == needle
}

#[cfg(any(feature = "search", test))]
fn push_cfg_label(labels: &mut BTreeSet<String>, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if !matches!(token.as_str(), "cfg" | "all" | "any" | "not" | "values") {
        labels.insert(std::mem::take(token));
    } else {
        token.clear();
    }
}

fn dependency_table_facts(
    project_root: &Path,
    kind: CargoDependencyKind,
    target: Option<&str>,
    dependencies: &DepsSet,
) -> Vec<CargoDependencyFacts> {
    dependencies
        .iter()
        .map(|(name, dependency)| dependency_fact(project_root, name, dependency, kind, target))
        .collect()
}

fn dependency_fact(
    project_root: &Path,
    name: &str,
    dependency: &Dependency,
    kind: CargoDependencyKind,
    target: Option<&str>,
) -> CargoDependencyFacts {
    let mut features = dependency.req_features().to_vec();
    features.sort();
    features.dedup();
    let package_name = dependency.package().unwrap_or(name).to_string();
    let version_req = dependency
        .try_req()
        .ok()
        .map(ToOwned::to_owned)
        .or_else(|| workspace_dependency_version_req(project_root, name));
    CargoDependencyFacts {
        dependency_key: name.to_string(),
        import_name: name.replace('-', "_"),
        package_name,
        version_req,
        kind,
        target: target.map(compact_cfg_expression),
        optional: dependency.optional(),
        features,
    }
}

fn workspace_dependency_version_req(project_root: &Path, name: &str) -> Option<String> {
    project_root
        .ancestors()
        .skip(1)
        .filter_map(read_manifest)
        .find_map(|manifest| {
            manifest
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.dependencies.get(name))
                .and_then(|dependency| dependency.try_req().ok())
                .map(ToOwned::to_owned)
        })
}

fn compact_cfg_expression(expression: &str) -> String {
    expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
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
