//! Hook-oriented query command mapped onto provider-owned search views.

use super::query_options::{QueryOptions, QuerySearchOptions};
pub(super) use super::query_window::render_query_local_window;

pub(super) enum QueryCommand {
    Help,
    Search(QuerySearchOptions),
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = std::ffi::OsString>,
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
    Ok(QueryCommand::Search(search_options))
}

fn is_exact_direct_source_selector(selector: &str) -> bool {
    let selector = selector.strip_prefix("owner:").unwrap_or(selector);
    let path = selector.split(':').next().unwrap_or(selector);
    !path.is_empty()
        && !path.contains('*')
        && !path.contains('?')
        && !path.contains('[')
        && !path.contains('{')
}

pub(super) fn print_query_help() {
    println!(
        "rs-harness query <owner-path[:start:end]> [items tests] [--query SYMBOL] [--names-only | --code] [PROJECT_ROOT]\n\
rs-harness query --catalog <declarations|imports|calls|macros|cfg> [--json] [PROJECT_ROOT]\n\
rs-harness query --treesitter-query '<s-expression>' [--selector <path[:line|:start:end]>] [--term TERM...] [--code] [--json] [PROJECT_ROOT]\n\
rs-harness query --from-hook direct-source-read --selector <path[:line-range]> --code [PROJECT_ROOT]\n\
rs-harness query --from-hook KIND --selector SELECTOR [--query SYMBOL | --term TERM] [--names-only | --code] [PROJECT_ROOT]\n\
rs-harness query --term TERM [--term TERM...] [--surface PIPE] [--view seeds] [PROJECT_ROOT]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Concrete Rust owner selectors route to search owner items/tests; multi-term queries route to search fzf query-set.\n\
Tree-sitter-compatible syntax catalog and inline queries emit semantic-tree-sitter-query.v1 packets through the normal query command.\n\
Glob or broad selectors without terms route to search prime --view seeds.\n\
Owner item queries emit |query status=hit|miss match=exact|fallback-contains|none.\n\
Use --code after selecting an owner/symbol or hook path/range to emit compact parser-owned code."
    );
}
