use std::path::Path;

use crate::RustProjectHarnessScope;
use crate::parser::{ParsedRustModule, RustReasoningOwnerBranchFacts};

use super::RustSearchOptions;
use super::context::PackageSearchContext;
use super::format::display_project_path;

pub(super) fn module_allowed(
    context: &PackageSearchContext,
    module: &ParsedRustModule,
    options: &RustSearchOptions,
) -> bool {
    options
        .owner
        .as_deref()
        .is_none_or(|owner| owner_path_matches(&context.package_root, &module.report.path, owner))
        && options.scope.as_deref().is_none_or(|scope| {
            scope == "all"
                || module_is_scope(&context.scope, module, scope)
                || owner_path_matches(&context.package_root, &module.report.path, scope)
        })
}

pub(super) fn module_is_scope(
    scope: &RustProjectHarnessScope,
    module: &ParsedRustModule,
    scope_name: &str,
) -> bool {
    match scope_name {
        "src" => scope
            .source_paths
            .iter()
            .any(|path| module.report.path.starts_with(path)),
        "tests" => scope
            .test_paths
            .iter()
            .any(|path| module.report.path.starts_with(path)),
        "benches" => module
            .report
            .path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "benches"),
        "examples" => module
            .report
            .path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "examples"),
        "build" => {
            module
                .report
                .path
                .file_name()
                .and_then(|name| name.to_str())
                == Some("build.rs")
        }
        _ => false,
    }
}

pub(super) fn owner_branch_matches(
    package_root: &Path,
    branch: &RustReasoningOwnerBranchFacts,
    query: &str,
) -> bool {
    owner_path_matches(package_root, &branch.path, query)
        || branch.owner_namespace.join("/").contains(query)
        || branch.owner_namespace.join("::").contains(query)
}

pub(super) fn owner_path_matches(package_root: &Path, path: &Path, query: &str) -> bool {
    let display = display_project_path(package_root, path);
    display == query
        || display.ends_with(query)
        || display.contains(query)
        || query_matches_absolute_path_from_ancestor(package_root, path, query)
}

fn query_matches_absolute_path_from_ancestor(
    package_root: &Path,
    path: &Path,
    query: &str,
) -> bool {
    let query_path = Path::new(query);
    !query_path.is_absolute()
        && package_root
            .ancestors()
            .any(|ancestor| ancestor.join(query_path) == path)
}
