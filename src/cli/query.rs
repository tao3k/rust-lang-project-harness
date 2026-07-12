//! Hook-oriented query command mapped onto provider-owned search views.

use std::ffi::OsString;

use super::query_options::{QueryOptions, QuerySearchOptions};
pub(super) use super::query_window::{render_query_local_item_code, render_query_local_window};

pub(super) enum QueryCommand {
    Help,
    Search(Box<QuerySearchOptions>),
}

pub(super) fn query_guide_kind(args: &[OsString]) -> bool {
    args.first().and_then(|arg| arg.to_str()) == Some("guide")
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = OsString>,
) -> Result<QueryCommand, String> {
    let options = QueryOptions::parse(args)?;
    if options.help {
        return Ok(QueryCommand::Help);
    }
    let wants_direct_source_items = options.from_hook.as_deref() == Some("direct-source-read")
        && options.query.is_none()
        && options.terms.is_empty()
        && options
            .selector
            .as_deref()
            .is_some_and(is_exact_direct_source_selector);
    let mut search_options = options.search_options()?;
    if wants_direct_source_items && !search_options.pipes.iter().any(|pipe| pipe == "items") {
        search_options.pipes.push("items".to_string());
    }
    if wants_direct_source_items && search_options.output_view.as_deref() != Some("read-packet") {
        search_options.output_view = None;
    }
    Ok(QueryCommand::Search(Box::new(search_options)))
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
        "rs-harness query <owner-path[:start:end]> [items tests] [--query SYMBOL] [--names-only | --code] [--workspace WORKSPACE]\n\
rs-harness query --catalog flow-lite --where 'source.call=NAME sink.constructs=TYPE scope.fn=FUNCTION' [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
rs-harness query --from-hook direct-source-read --selector <path[:line-range]> [--workspace WORKSPACE] [--source worktree|index|head] --code\n\
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
Flow-lite query forms accept one positional workspace root for ABI corpus compatibility.\n\
Use --source only to choose worktree, index, or head content.\n\
Use --code after selecting an owner/symbol or hook path/range to emit compact parser-owned code."
    );
}
