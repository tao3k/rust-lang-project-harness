//! Hook-oriented query command mapped onto provider-owned search views.

use std::ffi::OsString;

use super::query_options::{QueryOptions, QuerySearchOptions};
pub(super) use super::query_window::{render_query_local_item_code, render_query_local_window};

pub(super) enum QueryCommand {
    Help,
    Search(Box<QuerySearchOptions>),
}

pub(super) enum QueryGuideKind {
    Query,
    TreeSitter,
}

pub(super) fn query_guide_kind(args: &[OsString]) -> Option<QueryGuideKind> {
    if args.first().and_then(|arg| arg.to_str()) != Some("guide") {
        return None;
    }
    if args
        .iter()
        .skip(1)
        .any(|arg| matches!(arg.to_str(), Some("treesitter" | "tree-sitter")))
    {
        Some(QueryGuideKind::TreeSitter)
    } else {
        Some(QueryGuideKind::Query)
    }
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

|mode names command="query <owner-path> --query <symbol> --names-only" output=item-names
|mode frontier command="query <owner-path> --query <symbol>" output=item-frontier code=false
|mode code command="query <owner-path> --query <symbol> --workspace <WORKSPACE> --code" output=pure-code requires=unique-match
|mode exact-range command="query --from-hook direct-source-read --selector <path:start-end> --code" output=pure-code maxWindow=40
|mode workspace-range command="query --from-hook direct-source-read --selector <workspace-path:start-end> --workspace <WORKSPACE> --code" output=pure-code
|mode read-plan trigger="wide-selector|low-signal-window|broad-selector" output=read-frontier code=false

|action item.code mapsTo="query <owner-path> --query <item-name> --workspace <WORKSPACE> --code"
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

pub(super) fn print_tree_sitter_query_guide() {
    println!(
        r#"[treesitter-query-guide] lang=rust engine=tree-sitter protocol=treesitter-query-guide.v1 root=.
|contract base=tree-sitter native-extension=rs-harness
|contract no-code-default=true output=capture-frontier
|contract code-output=true requires="exact --selector" output=pure-code reason="syntax query locates; exact selector extracts"
|contract codeTargetDefault=enclosing-item fallback=pattern-root captureText=false

|syntax pattern=s-expression captures=@name fields=name:,type:,value: predicates=#eq?,#match?,#any-of?
|syntax location=path,lineRange,startByte,endByte
|syntax nodeFields=nodeType,field,capture,text,range,patternRoot,enclosingItem
|native extension=ownerPath,symbolKind,itemRange,visibility,doc,tests,frontier,exactRead

|template id=rust.functions pattern="(function_item name: (identifier) @function.name)" capture=function.name target=enclosing-item
|template id=rust.structs pattern="(struct_item name: (type_identifier) @type.name)" capture=type.name target=enclosing-item
|template id=rust.enums pattern="(enum_item name: (type_identifier) @type.name)" capture=type.name target=enclosing-item
|template id=rust.impls pattern="(impl_item type: (_) @impl.target)" capture=impl.target target=pattern-root
|template id=rust.calls pattern="(call_expression function: (_) @call.target)" capture=call.target target=hot-block
|template id=rust.macros pattern="(macro_invocation macro: (identifier) @macro.name)" capture=macro.name target=pattern-root
|template id=rust.tests pattern="(attribute_item) @attr (#match? @attr \"test\")" capture=attr target=enclosing-item

|mode frontier command="query --treesitter-query <pattern> --workspace <workspace-root>" output=capture-frontier code=false
|mode scoped-frontier command="query --selector <path-or-range> --treesitter-query <pattern> --workspace <workspace-root>" output=capture-frontier code=false
|mode exact-code command="query --selector <path-or-range> --treesitter-query <pattern> --workspace <workspace-root> --code" output=pure-code
|mode strict command="... --strict-treesitter" noMatch=fail stdout=empty

|rule multiMatchWithoutCode ok=true cap=12 output=frontier
|rule codeWithTreeSitterWithoutSelector exit=nonzero reason="exact selector required before pure code"
|rule codeWithTreeSitterMultiMatch exit=nonzero reason="narrow selector, predicate, or pattern before pure code"
|rule noMatch default=frontier-empty strict=false
|rule noMatchStrict exit=nonzero stdout=empty

|example frontier="query --treesitter-query '(function_item name: (identifier) @function.name)' --workspace <workspace-root>"
|example exactCode="query --selector src/cli/query.rs --treesitter-query '(function_item name: (identifier) @function.name (#eq? @function.name \"parse_query\"))' --workspace <workspace-root> --code"
|avoid broad-code-output,capture-name-only-by-default,inline-metadata-in-code-stdout"#
    );
}

pub(super) fn print_query_help() {
    println!(
        "rs-harness query <owner-path[:start:end]> [items tests] [--query SYMBOL] [--names-only | --code] [--workspace WORKSPACE]\n\
rs-harness query --catalog flow-lite --where 'source.call=NAME sink.constructs=TYPE scope.fn=FUNCTION' [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
rs-harness query --catalog <declarations|imports|calls|macros|cfg> [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
rs-harness query --treesitter-query '<s-expression>' [<workspace-root>] [--selector <path[:line|:start:end]>] [--term TERM...] [--workspace WORKSPACE] [--code] [--json]\n\
rs-harness query --from-hook direct-source-read --selector <path[:line-range]> [--workspace WORKSPACE] [--source worktree|index|head] --code\n\
rs-harness query --from-hook KIND --selector SELECTOR [--query SYMBOL | --term TERM] [--names-only | --code] [--workspace WORKSPACE]\n\
rs-harness search fzf TERM owner [--view seeds] [--workspace WORKSPACE]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Concrete Rust owner selectors route to search owner items/tests; workspace term discovery is the explicit search fzf surface.\n\
Tree-sitter-compatible syntax catalog and inline queries emit semantic-tree-sitter-query.v1 packets through the normal query command.\n\
Flow-lite native relation queries emit compact locator/provenance frontiers or semantic-flow-lite.v1 JSON without running CodeQL.\n\
Glob or broad selectors without terms route to search prime --view seeds.\n\
Owner item queries emit |query status=hit|miss match=exact|fallback-contains|none.\n\
Use --workspace WORKSPACE when the selector is workspace-relative; owner and direct-source query forms never accept a trailing workspace root.\n\
Catalog, tree-sitter, and flow-lite query forms also accept one positional workspace root for ABI corpus compatibility.\n\
Use --source only to choose worktree, index, or head content.\n\
Use --code after selecting an owner/symbol or hook path/range to emit compact parser-owned code."
    );
}
