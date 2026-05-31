use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::parser::{parse_cargo_cfg_facts, parse_cargo_dependency_facts, parse_cargo_manifest};

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    append_block, compact_locations, display_project_path, empty_dash, package_label,
    package_roots_for_request, render_cargo_dependency_line,
};
use super::hits::{OwnerHit, dependency_usage, matching_dependencies, text_hits};
use super::limits::SEARCH_HIT_LIMIT;
use super::owner::public_api_lines_for_dependency;

pub(super) fn render_search_workspace(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = format!(
        "[search-workspace] root={} pkg={}\n",
        display_project_path(project_root, project_root),
        roots.len()
    );
    for package_root in roots.into_iter().take(SEARCH_HIT_LIMIT) {
        let _ = writeln!(
            rendered,
            "|package {} root={} manifest={} source=manifest manager=cargo next=prime",
            package_label(project_root, &package_root),
            display_project_path(project_root, &package_root),
            display_project_path(project_root, &package_root.join("Cargo.toml"))
        );
    }
    Ok(rendered)
}

pub(super) fn render_search_targets(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in roots {
        let manifest = parse_cargo_manifest(&package_root);
        let mut block = format!(
            "[search-targets] pkg={} source={} test={} bench={} example={}\n",
            package_label(project_root, &package_root),
            manifest.source_target_files.len(),
            manifest.test_target_files.len(),
            manifest.bench_targets.len(),
            manifest.example_target_files.len()
        );
        for path in manifest
            .source_target_files
            .iter()
            .chain(manifest.test_target_files.iter())
            .chain(manifest.example_target_files.iter())
            .take(SEARCH_HIT_LIMIT)
        {
            let _ = writeln!(
                block,
                "|target path={} source=manifest manager=cargo next=owner:{}",
                display_project_path(&package_root, path),
                display_project_path(&package_root, path)
            );
        }
        for bench in manifest.bench_targets.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|target bench:{} path={} harness={} required_features={} source=manifest manager=cargo",
                bench.name,
                display_project_path(&package_root, &bench.path),
                bench.harness,
                empty_dash(&bench.required_features)
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_deps(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match query {
        Some(query) => render_search_dep(project_root, config, query, options),
        None => render_search_deps_list(project_root, config, options),
    }
}

fn render_search_deps_list(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let mut block = format!(
            "[search-deps] pkg={} dep={}\n",
            package_label(project_root, &context.package_root),
            context.cargo_dependencies.len()
        );
        for dependency in context.cargo_dependencies.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_dep(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let parsed_query = DependencySearchQuery::parse(query);
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let deps = matching_dependencies(&context.cargo_dependencies, &parsed_query.dependency);
        let version_scope = parsed_query
            .requested_version
            .as_deref()
            .map(|version| dependency_version_scope(&deps, version))
            .unwrap_or(DependencyVersionScope::Current);
        let dependency_usage = if version_scope == DependencyVersionScope::Current {
            dependency_usage(&context, &parsed_query.dependency)
        } else {
            Vec::new()
        };
        let path_usage = parsed_query.subpath.as_deref().map_or_else(
            || dependency_usage.clone(),
            |subpath| {
                dependency_subpath_usage(
                    &context,
                    &dependency_usage,
                    &parsed_query.dependency,
                    subpath,
                )
            },
        );
        let usage = parsed_query.api.as_deref().map_or_else(
            || path_usage.clone(),
            |api| dependency_api_usage(&context, &path_usage, api),
        );
        let public_api = if version_scope == DependencyVersionScope::Current
            && options.pipes.iter().any(|pipe| pipe == "public-api")
        {
            public_api_lines_for_dependency(&context, &parsed_query.dependency, &usage)
        } else {
            Vec::new()
        };
        let mut block = format!(
            "[search-deps] q={} pkg={} dep={} own={} api={}",
            query,
            package_label(project_root, &context.package_root),
            deps.len(),
            usage.len(),
            public_api.len()
        );
        if let Some(requested_version) = parsed_query.requested_version.as_deref() {
            let _ = write!(
                block,
                " requestedVersion={} versionScope={}",
                requested_version,
                version_scope.label()
            );
        }
        if let Some(subpath) = parsed_query.subpath.as_deref() {
            let _ = write!(block, " subpath={subpath}");
        }
        if let Some(api_query) = parsed_query.api.as_deref() {
            let _ = write!(block, " apiQuery={api_query}");
        }
        block.push('\n');
        for dependency in deps.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        if version_scope == DependencyVersionScope::External {
            let _ = writeln!(
                block,
                "|note requestedVersion={} scope=external currentWorkspaceVersion={} next={}",
                parsed_query.requested_version.as_deref().unwrap_or("-"),
                current_workspace_versions(&context.cargo_dependencies, &parsed_query.dependency),
                parsed_query.external_next()
            );
        }
        for hit in usage.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hitKind={}{} locations={} next=owner:{}",
                display_project_path(&context.package_root, &hit.path),
                parsed_query.owner_hit_kind(),
                parsed_query.owner_metadata(),
                compact_locations(&hit.locations),
                display_project_path(&context.package_root, &hit.path)
            );
        }
        for line in public_api.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{line}");
        }
        let _ = writeln!(block, "|next {}", parsed_query.next_actions(version_scope));
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

#[derive(Debug)]
struct DependencySearchQuery {
    dependency: String,
    subpath: Option<String>,
    requested_version: Option<String>,
    api: Option<String>,
}

impl DependencySearchQuery {
    fn parse(query: &str) -> Self {
        let (dependency_and_version, api) = query
            .split_once("::")
            .map_or((query, None), |(dependency, api)| (dependency, Some(api)));
        let (dependency_path, requested_version) = dependency_and_version
            .split_once('@')
            .map_or((dependency_and_version, None), |(dependency, version)| {
                (dependency, Some(version))
            });
        let (dependency, subpath) = dependency_path
            .split_once('/')
            .map_or((dependency_path, None), |(dependency, subpath)| {
                (dependency, Some(subpath))
            });
        Self {
            dependency: dependency.trim().to_string(),
            subpath: subpath
                .map(str::trim)
                .filter(|subpath| !subpath.is_empty())
                .map(ToOwned::to_owned),
            requested_version: requested_version
                .map(str::trim)
                .filter(|version| !version.is_empty())
                .map(ToOwned::to_owned),
            api: api
                .map(str::trim)
                .filter(|api| !api.is_empty())
                .map(ToOwned::to_owned),
        }
    }

    fn next_actions(&self, version_scope: DependencyVersionScope) -> String {
        let docs_dependency = self.docs_dependency(version_scope);
        if let Some(api) = self.api.as_deref() {
            format!(
                "dependency:{},docs-use:{}::{},text:{},tests:{}",
                self.dependency, docs_dependency, api, api, api
            )
        } else {
            format!(
                "dependency:{},docs-use:{},import:{},tests",
                self.dependency, docs_dependency, self.dependency
            )
        }
    }

    fn external_next(&self) -> String {
        let dependency = self.docs_dependency(DependencyVersionScope::External);
        self.api.as_deref().map_or_else(
            || format!("docs-use:{dependency}"),
            |api| format!("docs-use:{dependency}::{api}"),
        )
    }

    fn docs_dependency(&self, version_scope: DependencyVersionScope) -> String {
        let dependency = self.subpath.as_deref().map_or_else(
            || self.dependency.clone(),
            |subpath| format!("{}/{subpath}", self.dependency),
        );
        if version_scope == DependencyVersionScope::External
            && let Some(version) = self.requested_version.as_deref()
        {
            return format!("{dependency}@{version}");
        }
        dependency
    }

    fn owner_hit_kind(&self) -> &'static str {
        if self.api.is_some() {
            "dependency-api"
        } else if self.subpath.is_some() {
            "dependency-path"
        } else {
            "dependency"
        }
    }

    fn owner_metadata(&self) -> String {
        let mut metadata = String::new();
        if let Some(subpath) = self.subpath.as_deref() {
            let _ = write!(metadata, " subpath={subpath}");
        }
        if let Some(api) = self.api.as_deref() {
            let _ = write!(metadata, " apiQuery={api}");
        }
        metadata
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyVersionScope {
    Current,
    External,
}

impl DependencyVersionScope {
    fn label(self) -> &'static str {
        match self {
            DependencyVersionScope::Current => "current",
            DependencyVersionScope::External => "external",
        }
    }
}

fn dependency_version_scope(
    dependencies: &[&crate::parser::CargoDependencyFacts],
    requested_version: &str,
) -> DependencyVersionScope {
    if dependencies
        .iter()
        .any(|dependency| dependency.version_req.as_deref() == Some(requested_version))
    {
        DependencyVersionScope::Current
    } else {
        DependencyVersionScope::External
    }
}

fn current_workspace_versions(
    dependencies: &[crate::parser::CargoDependencyFacts],
    dependency: &str,
) -> String {
    let versions = matching_dependencies(dependencies, dependency)
        .into_iter()
        .filter_map(|dependency| dependency.version_req.as_deref())
        .collect::<Vec<_>>();
    if versions.is_empty() {
        "-".to_string()
    } else {
        versions.join(",")
    }
}

fn dependency_api_usage(
    context: &PackageSearchContext,
    dependency_usage: &[OwnerHit],
    api: &str,
) -> Vec<OwnerHit> {
    let dependency_owner_paths = dependency_usage
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    let mut grouped = BTreeMap::<PathBuf, Vec<String>>::new();
    for module in &context.parsed_modules {
        if !dependency_owner_paths.contains(&module.report.path) {
            continue;
        }
        for (index, line) in module.source.lines().enumerate() {
            if line.contains(api) {
                grouped
                    .entry(module.report.path.clone())
                    .or_default()
                    .push(format!("{}:1", index + 1));
            }
        }
    }
    grouped
        .into_iter()
        .map(|(path, mut locations)| {
            locations.sort();
            locations.dedup();
            OwnerHit { path, locations }
        })
        .collect()
}

fn dependency_subpath_usage(
    context: &PackageSearchContext,
    dependency_usage: &[OwnerHit],
    dependency: &str,
    subpath: &str,
) -> Vec<OwnerHit> {
    let dependency_owner_paths = dependency_usage
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    let rust_path = format!("{dependency}::{}", subpath.replace('/', "::"));
    let mut grouped = BTreeMap::<PathBuf, Vec<String>>::new();
    for module in &context.parsed_modules {
        if !dependency_owner_paths.contains(&module.report.path) {
            continue;
        }
        for (index, line) in module.source.lines().enumerate() {
            if line.contains(&rust_path) {
                grouped
                    .entry(module.report.path.clone())
                    .or_default()
                    .push(format!("{}:1", index + 1));
            }
        }
    }
    grouped
        .into_iter()
        .map(|(path, mut locations)| {
            locations.sort();
            locations.dedup();
            OwnerHit { path, locations }
        })
        .collect()
}

pub(super) fn render_search_features(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match query {
        Some(query) => render_search_feature(project_root, config, query, options),
        None => render_search_features_list(project_root, config, options),
    }
}

fn render_search_features_list(
    project_root: &Path,
    config: &RustHarnessConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in roots {
        let features = manifest_features(&package_root);
        let mut block = format!(
            "[search-features] pkg={} feat={}\n",
            package_label(project_root, &package_root),
            features.len()
        );
        for (name, enables) in features.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|feature {} enables={} source=manifest manager=cargo",
                name,
                empty_dash(enables)
            );
        }
        if !features.is_empty() {
            let next = features
                .iter()
                .take(4)
                .map(|(name, _)| format!("features:{name}"))
                .collect::<Vec<_>>();
            let _ = writeln!(block, "|next {}", next.join(","));
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_feature(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in roots {
        let features = manifest_features(&package_root);
        let enables = features
            .iter()
            .find(|(name, _)| name == query)
            .map(|(_, enables)| enables.clone())
            .unwrap_or_default();
        let dependencies = parse_cargo_dependency_facts(&package_root);
        let feature_deps = dependencies
            .iter()
            .filter(|dependency| {
                enables.iter().any(|enabled| {
                    enabled == &format!("dep:{}", dependency.dependency_key)
                        || enabled == &dependency.dependency_key
                }) || dependency.features.iter().any(|feature| feature == query)
            })
            .collect::<Vec<_>>();
        let mut block = format!(
            "[search-features] q={} pkg={} feat={} dep={}\n",
            query,
            package_label(project_root, &package_root),
            usize::from(features.iter().any(|(name, _)| name == query)),
            feature_deps.len()
        );
        let _ = writeln!(
            block,
            "|feature {} enables={} source=manifest manager=cargo",
            query,
            empty_dash(&enables)
        );
        for dependency in feature_deps.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        let _ = writeln!(block, "|next cfg:{query},text:{query}(scope=src),tests");
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_cfg(
    project_root: &Path,
    config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let mut cfg_options = options.clone();
    cfg_options.scope.get_or_insert_with(|| "all".to_string());
    let contexts = search_contexts(project_root, config, &cfg_options)?;
    let mut rendered = String::new();
    for context in contexts {
        let cfgs = parse_cargo_cfg_facts(&context.package_root)
            .into_iter()
            .filter(|cfg| {
                cfg.cfg == query
                    || cfg.cfg.strip_prefix("feature:") == Some(query)
                    || cfg.expression.contains(query)
                    || cfg.declared_in.contains(query)
            })
            .collect::<Vec<_>>();
        let dependencies = parse_cargo_dependency_facts(&context.package_root)
            .into_iter()
            .filter(|dependency| {
                dependency
                    .target
                    .as_deref()
                    .is_some_and(|target| target.contains(query))
            })
            .collect::<Vec<_>>();
        let source_hits = text_hits(&context, query, &cfg_options);
        let mut block = format!(
            "[search-cfg] q={} pkg={} cfg={} dep={} own={}\n",
            query,
            package_label(project_root, &context.package_root),
            cfgs.len(),
            dependencies.len(),
            source_hits.len()
        );
        for cfg in cfgs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|cfg {} declared_in={} expr={} source=manifest manager=cargo next=text:{}(scope=src)",
                cfg.cfg, cfg.declared_in, cfg.expression, query
            );
        }
        for dependency in dependencies.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(&dependency));
        }
        for hit in source_hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hit_kind=cfg locations={} next=owner:{}",
                display_project_path(&context.package_root, &hit.path),
                compact_locations(&hit.locations),
                display_project_path(&context.package_root, &hit.path)
            );
        }
        let _ = writeln!(
            block,
            "|next text:cfg({query})(scope=src),text:{query}(scope=tests)"
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn manifest_features(package_root: &Path) -> Vec<(String, Vec<String>)> {
    let Ok(content) = fs::read_to_string(package_root.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Ok(value) = content.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(features) = value.get("features").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    let mut parsed = features
        .iter()
        .map(|(name, value)| {
            let enables = value
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (name.clone(), enables)
        })
        .collect::<Vec<_>>();
    parsed.sort_by(|left, right| left.0.cmp(&right.0));
    parsed
}
