//! Hook-oriented query command mapped onto provider-owned search views.

use std::ffi::OsString;

use super::query_options::QueryOptions;

pub(super) enum QueryCommand {
    Help,
    ExactSource(ExactSourceQuery),
    TreeSitter(Box<TreeSitterQuery>),
}

pub(super) struct ExactSourceQuery {
    pub(crate) selector: String,
    pub(crate) source_snapshot_envelope: Option<std::path::PathBuf>,
    pub(crate) json: bool,
    pub(crate) code: bool,
    pub(crate) names_only: bool,
    pub(crate) provider_id: Option<String>,
    pub(crate) parser_identity_digest: Option<String>,
    pub(crate) query_pack_digest: Option<String>,
}

#[derive(Default)]
struct ExactQueryAuthority {
    source_snapshot_envelope: Option<std::path::PathBuf>,
    provider_id: Option<String>,
    parser_identity_digest: Option<String>,
    query_pack_digest: Option<String>,
}

impl ExactQueryAuthority {
    fn is_empty(&self) -> bool {
        self.source_snapshot_envelope.is_none()
            && self.provider_id.is_none()
            && self.parser_identity_digest.is_none()
            && self.query_pack_digest.is_none()
    }
}

pub(super) struct TreeSitterQuery {
    pub(crate) source: Option<String>,
    pub(crate) catalog_id: Option<String>,
    pub(crate) selector: Option<String>,
    pub(crate) captures: Vec<String>,
    pub(crate) node_types: Vec<String>,
    pub(crate) fields: Vec<String>,
    pub(crate) predicates_json: Option<String>,
    pub(crate) workspace_root: std::path::PathBuf,
    pub(crate) json: bool,
    pub(crate) code: bool,
    pub(crate) provider_id: Option<String>,
    pub(crate) parser_identity_digest: Option<String>,
    pub(crate) query_pack_digest: Option<String>,
}

pub(super) fn query_guide_kind(args: &[OsString]) -> bool {
    args.first().and_then(|arg| arg.to_str()) == Some("guide")
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = OsString>,
) -> Result<QueryCommand, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if let Some(options) = parse_tree_sitter_query(&args)? {
        return Ok(QueryCommand::TreeSitter(Box::new(options)));
    }
    let (args, authority) = extract_exact_query_authority(args)?;
    let options = QueryOptions::parse(args)?;
    if options.help {
        return Ok(QueryCommand::Help);
    }
    let wants_direct_source_items = options
        .selector
        .as_deref()
        .is_some_and(is_exact_direct_source_selector);
    if wants_direct_source_items {
        let selector = options
            .selector
            .clone()
            .ok_or_else(|| "exact source query requires a selector".to_string())?;
        return Ok(QueryCommand::ExactSource(ExactSourceQuery {
            selector,
            source_snapshot_envelope: authority.source_snapshot_envelope,
            json: options.json,
            code: options.code,
            names_only: options.names_only,
            provider_id: authority.provider_id,
            parser_identity_digest: authority.parser_identity_digest,
            query_pack_digest: authority.query_pack_digest,
        }));
    }
    if !authority.is_empty() {
        return Err(
            "source snapshot and typed projection identity options require an exact source selector"
                .to_string(),
        );
    }
    Err(
        "rust query requires an exact --selector; use `asp rust search owner <owner-path> items --query <symbol> --names-only --workspace .` for owner or symbol discovery"
            .to_string(),
    )
}

fn parse_tree_sitter_query(args: &[OsString]) -> Result<Option<TreeSitterQuery>, String> {
    let is_tree_sitter_query = args
        .iter()
        .any(|argument| matches!(argument.to_str(), Some("--treesitter-query" | "--catalog")));
    if !is_tree_sitter_query {
        return Ok(None);
    }
    let mut options = TreeSitterQuery {
        source: None,
        catalog_id: None,
        selector: None,
        captures: Vec::new(),
        node_types: Vec::new(),
        fields: Vec::new(),
        predicates_json: None,
        workspace_root: std::path::PathBuf::from("."),
        json: false,
        code: false,
        provider_id: None,
        parser_identity_digest: None,
        query_pack_digest: None,
    };
    let mut index = 0;
    while index < args.len() {
        let argument = args[index]
            .to_str()
            .ok_or_else(|| "tree-sitter query arguments must be UTF-8".to_string())?;
        match argument {
            "--treesitter-query" => {
                options.source = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--catalog" => {
                options.catalog_id = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--selector" => {
                options.selector = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--workspace" => {
                options.workspace_root =
                    std::path::PathBuf::from(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--asp-syntax-query-captures" => {
                options.captures = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-node-types" => {
                options.node_types = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-fields" => {
                options.fields = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-predicates-json" => {
                options.predicates_json =
                    Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--json" => options.json = true,
            "--code" => options.code = true,
            "--asp-provider-id" => {
                options.provider_id = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--asp-parser-identity-digest" => {
                options.parser_identity_digest =
                    Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--asp-query-pack-digest" => {
                options.query_pack_digest =
                    Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            _ => return Err(format!("unknown tree-sitter query option: {argument}")),
        }
        index += 1;
    }
    if options.source.is_some() == options.catalog_id.is_some() {
        return Err(
            "tree-sitter query requires exactly one of --treesitter-query or --catalog".to_string(),
        );
    }
    if options.code && options.selector.is_none() {
        return Err("tree-sitter query --code requires an exact --selector".to_string());
    }
    if options.code
        && options.json
        && (options.provider_id.is_none()
            || options.parser_identity_digest.is_none()
            || options.query_pack_digest.is_none())
    {
        return Err(
            "tree-sitter query --code --json requires typed provider and identity digests"
                .to_string(),
        );
    }
    Ok(Some(options))
}

fn tree_sitter_option_value(
    args: &[OsString],
    index: &mut usize,
    option: &str,
) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn tree_sitter_csv_option(
    args: &[OsString],
    index: &mut usize,
    option: &str,
) -> Result<Vec<String>, String> {
    Ok(tree_sitter_option_value(args, index, option)?
        .split(',')
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn extract_exact_query_authority(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<OsString>, ExactQueryAuthority), String> {
    let mut filtered = Vec::new();
    let mut authority = ExactQueryAuthority::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        if argument == "--source-snapshot-envelope" {
            if authority.source_snapshot_envelope.is_some() {
                return Err("--source-snapshot-envelope may be supplied only once".to_string());
            }
            let path = args.next().ok_or_else(|| {
                "--source-snapshot-envelope requires a JSON file path".to_string()
            })?;
            authority.source_snapshot_envelope = Some(std::path::PathBuf::from(path));
        } else if argument == "--asp-provider-id" {
            if authority.provider_id.is_some() {
                return Err("--asp-provider-id may be supplied only once".to_string());
            }
            authority.provider_id = Some(exact_query_option_value(&mut args, "--asp-provider-id")?);
        } else if argument == "--asp-parser-identity-digest" {
            if authority.parser_identity_digest.is_some() {
                return Err("--asp-parser-identity-digest may be supplied only once".to_string());
            }
            authority.parser_identity_digest = Some(exact_query_option_value(
                &mut args,
                "--asp-parser-identity-digest",
            )?);
        } else if argument == "--asp-query-pack-digest" {
            if authority.query_pack_digest.is_some() {
                return Err("--asp-query-pack-digest may be supplied only once".to_string());
            }
            authority.query_pack_digest = Some(exact_query_option_value(
                &mut args,
                "--asp-query-pack-digest",
            )?);
        } else {
            filtered.push(argument);
        }
    }
    Ok((filtered, authority))
}

fn exact_query_option_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))?
        .into_string()
        .map_err(|_| format!("{option} requires a UTF-8 value"))
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
        "rs-harness query --selector 'rust://OWNER#item/KIND/NAME' [--workspace WORKSPACE] [--source-snapshot-envelope JSON-FILE] [--names-only | --code]\n\
rs-harness query --treesitter-query QUERY [--workspace WORKSPACE]\n\
rs-harness query --catalog flow-lite --where 'source.call=NAME sink.constructs=TYPE scope.fn=FUNCTION' [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
rs-harness query --from-hook direct-source-read --selector 'rust://OWNER#item/KIND/NAME' [--workspace WORKSPACE] [--source-snapshot-envelope JSON-FILE] --code\n\
rs-harness query --from-hook KIND --selector SELECTOR --source-snapshot-envelope JSON-FILE --code --json --asp-provider-id ID --asp-parser-identity-digest DIGEST --asp-query-pack-digest DIGEST [--workspace WORKSPACE]\n\
rs-harness search dependency <crate-or-package> [items docs-use tests] [--view seeds] [--workspace WORKSPACE]\n\
rs-harness search guide [--workspace WORKSPACE]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Owner and symbol discovery is owned by `search owner`; `query` accepts only exact structural selectors or exact Tree-sitter/relation contracts.\n\
Dependency search is manifest-first: inspect Cargo.toml/Cargo.lock facts, import owners, public API/docs-use, and tests before web or docs.rs search.\n\
Flow-lite native relation queries emit compact locator/provenance frontiers or semantic-flow-lite.v1 JSON without running CodeQL.\n\
Use `asp rust search owner OWNER items --query SYMBOL --names-only --workspace .` to discover exact item selectors.\n\
Use --workspace WORKSPACE when the exact selector is workspace-relative; query never accepts an owner path as a positional discovery shortcut.\n\
Use --source-snapshot-envelope JSON-FILE with an exact selector to derive an editor-buffer Merkle root from asp.exact-source-snapshot-envelope.v1.\n\
Flow-lite query forms accept one positional workspace root for ABI corpus compatibility.\n\
Use --code only with an exact structural selector to emit compact parser-owned code."
    );
}

#[cfg(test)]
#[path = "../../tests/unit/cli/query/authority.rs"]
mod tests;
