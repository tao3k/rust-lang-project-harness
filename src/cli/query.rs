//! Hook-oriented query command mapped onto provider-owned search views.

use std::ffi::OsString;

use super::query_options::{QueryOptions, QuerySearchOptions};

pub(super) enum QueryCommand {
    Help,
    ExactSource(ExactSourceQuery),
    Search(Box<QuerySearchOptions>),
}

pub(super) struct ExactSourceQuery {
    pub(crate) selector: String,
    pub(crate) workspace_root: std::path::PathBuf,
    pub(crate) source_overlay: Option<std::path::PathBuf>,
    pub(crate) json: bool,
    pub(crate) code: bool,
    pub(crate) names_only: bool,
}

pub(super) fn query_guide_kind(args: &[OsString]) -> bool {
    args.first().and_then(|arg| arg.to_str()) == Some("guide")
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = OsString>,
) -> Result<QueryCommand, String> {
    let (args, source_overlay) = extract_source_overlay(args)?;
    let options = QueryOptions::parse(args)?;
    if options.help {
        return Ok(QueryCommand::Help);
    }
    let wants_direct_source_items = options.query.is_none()
        && options.terms.is_empty()
        && options
            .selector
            .as_deref()
            .is_some_and(is_exact_direct_source_selector);
    if wants_direct_source_items {
        let mut search_options = options.search_options()?;
        let selector = search_options
            .read_selector
            .take()
            .or_else(|| search_options.query.take())
            .ok_or_else(|| "exact source query requires a selector".to_string())?;
        return Ok(QueryCommand::ExactSource(ExactSourceQuery {
            selector,
            workspace_root: search_options
                .workspace_root
                .take()
                .unwrap_or_else(|| std::path::PathBuf::from(".")),
            source_overlay,
            json: search_options.json,
            code: search_options.item_code,
            names_only: search_options.item_names_only,
        }));
    }
    if source_overlay.is_some() {
        return Err("--source-overlay requires an exact source selector".to_string());
    }
    let search_options = options.search_options()?;
    Ok(QueryCommand::Search(Box::new(search_options)))
}

fn extract_source_overlay(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<OsString>, Option<std::path::PathBuf>), String> {
    let mut filtered = Vec::new();
    let mut source_overlay = None;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        if argument == "--source-overlay" {
            if source_overlay.is_some() {
                return Err("--source-overlay may be supplied only once".to_string());
            }
            let path = args
                .next()
                .ok_or_else(|| "--source-overlay requires a JSON file path".to_string())?;
            source_overlay = Some(std::path::PathBuf::from(path));
        } else {
            filtered.push(argument);
        }
    }
    Ok((filtered, source_overlay))
}

fn is_exact_direct_source_selector(selector: &str) -> bool {
    if selector.starts_with("rust://") && selector.contains("#item/") {
        return true;
    }
    let selector = selector.strip_prefix("owner:").unwrap_or(selector);
    let path = selector.split(':').next().unwrap_or(selector);
    !path.is_empty()
        && !path.contains('*')
        && !path.contains('?')
        && !path.contains('[')
        && !path.contains('{')
}

pub(super) fn print_query_guide() {
    println!(
        r#"[query-guide] lang=rust provider=asp-rust protocol=query-guide.v1 root=.
|contract stdout=frontier unless="--code + exact-selector|unique-match"
|contract pure-code when="--code + exact-selector|unique-match" header=false legend=false metadata=false
|contract no-inline-code-in-search default=true reason=search-is-discovery
|contract compact-projection editable=false use=understanding-only exactBeforePatch=true
|contract query-item-packet when="item-frontier|item-names" header="[query-item]" reason="query resolves item identity"
|contract search-owner-packet reservedFor="search owner discovery" header="[search-owner]"
|contract selector-hints fields=displayLineRange,sourceLocatorHint executable=false use=diagnostic-only

|mode names command="query <owner-path> --query <symbol> --names-only" output=query-item
|mode frontier command="query <owner-path> --query <symbol>" output=query-item code=false
|mode code command="query <owner-path> --query <symbol> --workspace <WORKSPACE> --code" output=pure-code requires=unique-match
|mode exact-range command="query --from-hook direct-source-read --selector <path:start-end> --code" output=pure-code maxWindow=40
|mode workspace-range command="query --from-hook direct-source-read --selector <workspace-path:start-end> --workspace <WORKSPACE> --code" output=pure-code
|mode read-plan trigger="wide-selector|low-signal-window|broad-selector" output=read-frontier code=false

|action item.code mapsTo="query <owner-path> --query <item-name> --workspace <WORKSPACE> --code"
|action item.exact-read mapsTo="query --selector <structural-selector> --workspace <WORKSPACE> --code"
|action window.code mapsTo="query --from-hook direct-source-read --selector <path:start-end> --code"
|action workspace-window.code mapsTo="query --from-hook direct-source-read --selector <workspace-path:start-end> --workspace <WORKSPACE> --code"
|action item.outline mapsTo="query <owner-path> --query <item-name> --view outline --workspace <WORKSPACE>"
|action exact-read mapsTo="query --from-hook direct-source-read --selector <exactRead> --workspace <WORKSPACE> --code"

|read-plan nodeKinds=range,window,symbol,hot
|read-plan relations=contains,split,remainder,matches,repairs
|read-plan required=rank,frontier,omit,avoid
|read-plan avoid=repeat-wide-read,manual-window-scan,raw-read

|output frontier-example="I=item:fn(parse_query)@src/cli/query.rs:32:54!code"
|output pure-code-example="stdout contains only source text"
|avoid inline-code-in-search,projection-as-edit-source,manual-window-scan,repeat-owner,raw-read"#
    );
}

pub(super) fn print_query_help() {
    println!(
        "rs-harness query <owner-path> [items tests] [--query SYMBOL] [--names-only | --code] [--workspace WORKSPACE] [--source-overlay JSON-FILE]\n\
rs-harness query --catalog flow-lite --where 'source.call=NAME sink.constructs=TYPE scope.fn=FUNCTION' [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
rs-harness query --from-hook direct-source-read --selector 'rust://OWNER#item/KIND/NAME' [--workspace WORKSPACE] [--source-overlay JSON-FILE] --code\n\
rs-harness query --from-hook KIND --selector SELECTOR [--query SYMBOL | --term TERM] [--names-only | --code] [--workspace WORKSPACE]\n\
rs-harness search dependency <crate-or-package> [items docs-use tests] [--view seeds] [--workspace WORKSPACE]\n\
rs-harness search guide [--workspace WORKSPACE]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Concrete Rust owner selectors route to search owner items/tests; workspace term discovery is owned by ASP `search lexical`.\n\
Dependency search is manifest-first: inspect Cargo.toml/Cargo.lock facts, import owners, public API/docs-use, and tests before web or docs.rs search.\n\
Flow-lite native relation queries emit compact locator/provenance frontiers or semantic-flow-lite.v1 JSON without running CodeQL.\n\
Glob or broad selectors without terms route to search prime --view seeds.\n\
Owner item queries emit |query status=hit|miss match=exact|fallback-contains|none.\n\
Use --workspace WORKSPACE when the selector is workspace-relative; owner and direct-source query forms never accept a trailing workspace root.\n\
Use --source-overlay JSON-FILE with an exact selector to derive an editor-buffer Merkle root from asp.source-overlay.v1.\n\
Flow-lite query forms accept one positional workspace root for ABI corpus compatibility.\n\
Use --code only with an exact structural selector or a unique owner/symbol match to emit compact parser-owned code."
    );
}
