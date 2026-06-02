use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::discovery::discover_cargo_package_roots;
use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, ParsedRustModule, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustTopLevelItemSyntax,
};
use crate::path::normalize_lexical_path;

use super::limits::SOURCE_LARGE_EFFECTIVE_LINES;

pub(super) fn render_owner_line(
    package_root: &Path,
    branch: &RustReasoningOwnerBranchFacts,
    parsed_module: Option<&ParsedRustModule>,
) -> String {
    let path = display_project_path(package_root, &branch.path);
    let mut parts = vec![
        format!("|owner {path}"),
        format!("role={}", owner_branch_role_labels(branch).join(",")),
    ];
    if !branch.owner_namespace.is_empty() {
        parts.push(format!("owner={}", branch.owner_namespace.join("/")));
    }
    let imports = compact_import_summary(&branch.import_summary);
    if !imports.is_empty() {
        parts.push(format!("imports={imports}"));
    }
    if parsed_module.is_some_and(|module| {
        module.source_metrics.effective_code_lines > SOURCE_LARGE_EFFECTIVE_LINES
    }) {
        parts.push("source_large=true".to_string());
        parts.push("next=items".to_string());
    } else {
        parts.push(format!("next=owner:{path}"));
    }
    parts.join(" ")
}

fn owner_branch_role_labels(branch: &RustReasoningOwnerBranchFacts) -> Vec<String> {
    branch
        .roles
        .iter()
        .map(|role| match role {
            RustReasoningOwnerBranchRole::Root => "root".to_string(),
            RustReasoningOwnerBranchRole::Facade => "facade".to_string(),
            RustReasoningOwnerBranchRole::Interface => "interface".to_string(),
            RustReasoningOwnerBranchRole::Binary => "binary".to_string(),
            RustReasoningOwnerBranchRole::PackageEntrypoint => "package-entrypoint".to_string(),
            RustReasoningOwnerBranchRole::RepeatedNamespace(segments) => {
                format!("repeated:{}", segments.join(","))
            }
            RustReasoningOwnerBranchRole::Branch => "branch".to_string(),
        })
        .collect()
}

fn compact_import_summary(imports: &crate::parser::RustReasoningImportFacts) -> String {
    let mut parts = Vec::new();
    push_count(&mut parts, "crate", imports.crate_imports);
    push_count(&mut parts, "self", imports.self_imports);
    push_count(&mut parts, "parent", imports.parent_imports);
    push_count(&mut parts, "external", imports.external_imports);
    push_count(&mut parts, "absolute", imports.absolute_imports);
    push_count(&mut parts, "glob", imports.glob_imports);
    push_count(&mut parts, "deep", imports.deep_relative_imports);
    parts.join(",")
}

fn push_count(parts: &mut Vec<String>, label: &str, count: usize) {
    if count > 0 {
        parts.push(format!("{label}:{count}"));
    }
}

pub(super) fn render_cargo_dependency_line(dependency: &CargoDependencyFacts) -> String {
    let mut parts = vec![
        format!("|dep {}", dependency.dependency_key),
        format!("import={}", dependency.import_name),
        format!("pkg={}", dependency.package_name),
        format!(
            "version={}",
            dependency.version_req.as_deref().unwrap_or("-")
        ),
        format!("kind={}", dependency_kind_label(dependency.kind)),
        format!("opt={}", dependency.optional),
        "source=manifest".to_string(),
        "manager=cargo".to_string(),
    ];
    if let Some(target) = &dependency.target {
        parts.push(format!("target={target}"));
    }
    parts.push(format!("feat={}", empty_dash(&dependency.features)));
    parts.join(" ")
}

fn dependency_kind_label(kind: CargoDependencyKind) -> &'static str {
    match kind {
        CargoDependencyKind::Normal => "normal",
        CargoDependencyKind::Dev => "dev",
        CargoDependencyKind::Build => "build",
    }
}

pub(super) fn render_item_line(item: &RustTopLevelItemSyntax) -> String {
    let name = item_display_name(item);
    let mut fields = vec![format!("|item {name}"), format!("kind={}", item.kind)];
    if item.is_public {
        fields.push("public=true".to_string());
    }
    if item.has_doc {
        fields.push("doc=true".to_string());
    }
    fields.push(format!("next=symbol:{name}"));
    fields.join(" ")
}

fn item_display_name(item: &RustTopLevelItemSyntax) -> &str {
    item.name
        .as_deref()
        .or(item.impl_target_name.as_deref())
        .unwrap_or("-")
}

pub(super) fn render_item_line_with_read(
    package_root: &Path,
    path: &Path,
    item: &RustTopLevelItemSyntax,
) -> String {
    let read_path = display_project_path(package_root, path);
    format!(
        "{} read={}:{}:{}",
        render_item_line(item),
        read_path,
        item.line,
        item.end_line
    )
}

pub(super) fn render_public_api_line(
    package_root: &Path,
    path: &Path,
    dependency: &str,
    item: &RustTopLevelItemSyntax,
) -> Option<String> {
    let name = item.name.as_deref().or(item.function_name.as_deref())?;
    Some(format!(
        "|api {} line={} dep={} kind={} name={} public={} doc={} reason=dependency-owner next=docs:{},tests",
        display_project_path(package_root, path),
        item.line,
        dependency,
        item.kind,
        name,
        item.is_public,
        item.has_doc,
        name
    ))
}

pub(super) fn empty_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(",")
    }
}

pub(super) fn compact_locations(locations: &[String]) -> String {
    if locations.is_empty() {
        return "-".to_string();
    }
    let mut selected = locations.iter().take(8).cloned().collect::<Vec<_>>();
    if locations.len() > 8 {
        selected.push(format!("+{}", locations.len() - 8));
    }
    selected.join(",")
}

pub(super) fn sort_locations(locations: &mut [String]) {
    locations.sort_by(|left, right| {
        location_sort_key(left)
            .cmp(&location_sort_key(right))
            .then_with(|| left.cmp(right))
    });
}

fn location_sort_key(location: &str) -> (usize, usize) {
    let mut parts = location.split(':');
    let line = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let column = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    (line, column)
}

pub(super) fn append_block(rendered: &mut String, block: &str) {
    if !rendered.is_empty() && !rendered.ends_with('\n') && !block.is_empty() {
        rendered.push('\n');
    }
    rendered.push_str(block);
}

pub(super) fn required_query<'a>(view: &str, query: Option<&'a str>) -> Result<&'a str, String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| format!("search {view} requires a query"))
}

pub(super) fn query_set_terms(query: &str) -> Vec<&str> {
    query
        .split(',')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect()
}

pub(super) fn owner_role_for_path(package_root: &Path, path: &Path) -> &'static str {
    let display = display_project_path(package_root, path);
    if display.starts_with("tests/") {
        "test"
    } else if display.starts_with("benches/") {
        "bench"
    } else if display.starts_with("examples/") {
        "example"
    } else {
        "source"
    }
}

pub(super) fn package_roots_for_request(
    project_root: &Path,
    config: &RustHarnessConfig,
    selected_package: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
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
    if let Some(selected_package) = selected_package {
        resolve_package_root(project_root, &package_roots, selected_package).map(|root| vec![root])
    } else {
        Ok(package_roots)
    }
}

pub(super) fn package_label(project_root: &Path, package_root: &Path) -> String {
    display_project_path(project_root, package_root)
}

pub(super) fn resolve_package_root(
    project_root: &Path,
    package_roots: &[PathBuf],
    selected_package: &str,
) -> Result<PathBuf, String> {
    let selected = selected_package.trim();
    package_roots
        .iter()
        .find(|package_root| {
            display_project_path(project_root, package_root) == selected
                || package_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == selected)
        })
        .cloned()
        .ok_or_else(|| format!("unknown package for search prime: {selected}"))
}

pub(super) fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}

pub(super) fn display_project_path(root: &Path, path: &Path) -> String {
    let root = normalize_lexical_path(root);
    let path = normalize_lexical_path(path);
    path.strip_prefix(&root)
        .map_or_else(|_| display_path(&path), display_path)
}

fn display_path(path: &Path) -> String {
    let rendered = path.display().to_string().replace('\\', "/");
    if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    }
}
