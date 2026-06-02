//! Public RFC search request and dispatch surface.

use std::path::Path;

use crate::RustHarnessConfig;

use super::{cargo, compact, dependency, format, owner, owner_view, prime, query};

/// Options shared by RFC search renderers.
#[derive(Debug, Clone, Default)]
pub struct RustSearchOptions {
    /// Optional package selector for workspaces.
    pub package: Option<String>,
    /// Optional owner/path selector for hit-level views.
    pub owner: Option<String>,
    /// Optional dependency selector for dependency-aware pattern recipes.
    pub dependency: Option<String>,
    /// Optional scope selector such as `src`, `tests`, or `all`.
    pub scope: Option<String>,
    /// Optional pipe stages after the source view.
    pub pipes: Vec<String>,
    /// Whether source lines may be emitted.
    pub lines: bool,
    /// Optional compact output view requested by the CLI.
    pub output_view: Option<String>,
    /// Optional seed limit requested by the CLI.
    pub seeds: Option<usize>,
    /// Optional exact-set query terms for providers that can merge same-view probes.
    pub query_set: Vec<String>,
    pub fzf_args: Vec<String>,
    /// Optional parser-native item query inside an owner/module result.
    pub item_query: Option<String>,
    /// Suppress compact code for item queries and render only parser item names.
    pub item_names_only: bool,
    /// Render only compact parser-owned code for item queries.
    pub item_code: bool,
}

/// Request object for rendering an RFC search view.
#[derive(Debug, Clone, Copy)]
pub struct RustSearchViewRequest<'a> {
    /// Project root to inspect.
    pub project_root: &'a Path,
    /// Harness configuration used for discovery and parser policy.
    pub config: &'a RustHarnessConfig,
    /// Search source view such as `prime`, `dependency`, or `deps`.
    pub view: &'a str,
    /// Optional query consumed by views such as `symbol` or `deps`.
    pub query: Option<&'a str>,
    /// Shared search filters and pipe stages.
    pub options: &'a RustSearchOptions,
}

/// Render an RFC search view by name.
///
/// # Errors
///
/// Returns an error when the project root or selected package cannot be
/// resolved, or when the view requires a query that was not supplied.
pub fn render_rust_project_harness_search_view_with_config(
    request: &RustSearchViewRequest<'_>,
) -> Result<String, String> {
    let project_root = request.project_root;
    let config = request.config;
    let options = request.options;
    let rendered = match request.view {
        "prime" => prime::render_search_prime(
            project_root,
            config,
            options.package.as_deref(),
            prime_seed_limit(options),
        ),
        "workspace" => cargo::render_search_workspace(project_root, config, options),
        "targets" => cargo::render_search_targets(project_root, config, options),
        "deps" => cargo::render_search_deps(project_root, config, request.query, options),
        "features" => cargo::render_search_features(project_root, config, request.query, options),
        "policy" => super::policy::render_search_policy(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "owner" => owner_view::render_search_owner(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "dependency" => dependency::render_search_dependency(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "tests" => owner::render_search_tests(project_root, config, request.query, options),
        "symbol" => query::render_search_symbol(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "callsite" => query::render_search_callsite(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "import" => query::render_search_import(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "query" => super::syntax_query::render_search_query(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "fzf" => query::render_search_fzf(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "cfg" => cargo::render_search_cfg(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "patterns" => Ok(query::render_search_patterns()),
        "pattern" => query::render_search_pattern(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "docs" => query::render_search_docs(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "docs-use" => query::render_search_docs_use(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "api" => query::render_search_api(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "public-external-types" => {
            query::render_search_public_external_types(project_root, config, options)
        }
        other => Err(format!("unknown search view: {other}")),
    }?;
    if options.output_view.as_deref() == Some("seeds") {
        Ok(rendered)
    } else {
        Ok(compact::compact_search_packet(&rendered))
    }
}

fn prime_seed_limit(options: &RustSearchOptions) -> Option<usize> {
    (options.output_view.as_deref() == Some("seeds")).then_some(options.seeds.unwrap_or(8))
}
