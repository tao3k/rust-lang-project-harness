use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::ExitCode;

use super::query::print_query_help;
use super::tree_sitter_query_locator::{SyntaxQuerySelector, parse_syntax_query_selector};
use super::tree_sitter_query_packet::{
    SyntaxQueryRow, syntax_query_matches_json, syntax_query_native_fact_refs,
};
use super::tree_sitter_query_projection::{
    SUPPORTED_TREE_SITTER_QUERY_NODES, SyntaxQueryPredicate, SyntaxQueryPredicateOp,
    SyntaxQueryPredicateValue, project_tree_sitter_query,
};

struct RustTreeSitterCatalog {
    id: &'static str,
    path: &'static str,
    source: &'static str,
    captures: &'static [&'static str],
    node_types: &'static [&'static str],
    fields: &'static [&'static str],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RustSyntaxQueryPlan {
    captures: Vec<String>,
    node_types: Vec<String>,
    fields: Vec<String>,
    predicates: Vec<SyntaxQueryPredicate>,
}

const RUST_TREE_SITTER_GRAMMAR_ID: &str = "tree-sitter-rust";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION: &str = "2026-06-04.v1";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_PATH: &str =
    "tree-sitter/tree-sitter-rust/grammar-profile.json";
const RUST_TREE_SITTER_GRAMMAR_PROFILE_SOURCE: &str =
    include_str!("../../tree-sitter/tree-sitter-rust/grammar-profile.json");
const MAX_SYNTAX_QUERY_GRAPH_ROWS: usize = 12;
const TREE_SITTER_QUERY_WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;
pub(super) fn run_tree_sitter_query_catalog(args: &[OsString]) -> Result<Option<ExitCode>, String> {
    if !args
        .iter()
        .any(|arg| arg == "--catalog" || arg == "--treesitter-query")
    {
        return Ok(None);
    }

    let mut catalog_id = None::<String>;
    let mut tree_sitter_query = None::<String>;
    let mut selector = None::<SyntaxQuerySelector>;
    let mut terms = Vec::<String>::new();
    let mut asp_plan = None::<RustSyntaxQueryPlan>;
    let mut json_output = false;
    let mut code_output = false;
    let mut workspace_root = None::<PathBuf>;
    let mut positionals = Vec::<PathBuf>::new();
    let mut pending_option = None::<String>;

    for arg in args {
        let value = arg
            .to_str()
            .ok_or_else(|| format!("query argument is not valid UTF-8: {arg:?}"))?;
        if let Some(option) = pending_option.take() {
            match option.as_str() {
                "--catalog" => {
                    catalog_id = Some(value.to_string());
                }
                "--treesitter-query" => {
                    tree_sitter_query = Some(value.to_string());
                }
                "--term" => {
                    terms.push(value.to_string());
                }
                "--selector" => {
                    selector = Some(parse_syntax_query_selector(value)?);
                }
                "--workspace" => {
                    if value.starts_with('-') {
                        return Err("--workspace requires a project root".to_string());
                    }
                    workspace_root = Some(PathBuf::from(value));
                }
                "--asp-syntax-query-captures" => {
                    asp_plan
                        .get_or_insert_with(RustSyntaxQueryPlan::default)
                        .captures = split_internal_plan_list(value);
                }
                "--asp-syntax-query-node-types" => {
                    asp_plan
                        .get_or_insert_with(RustSyntaxQueryPlan::default)
                        .node_types = split_internal_plan_list(value);
                }
                "--asp-syntax-query-fields" => {
                    asp_plan
                        .get_or_insert_with(RustSyntaxQueryPlan::default)
                        .fields = split_internal_plan_list(value);
                }
                "--asp-syntax-query-predicates-json" => {
                    asp_plan
                        .get_or_insert_with(RustSyntaxQueryPlan::default)
                        .predicates
                        .extend(parse_internal_query_predicates_json(value)?);
                }
                _ => unreachable!("unsupported pending query catalog option: {option}"),
            }
            continue;
        }

        match value {
            "--catalog" => {
                pending_option = Some(value.to_string());
            }
            "--treesitter-query" => {
                pending_option = Some(value.to_string());
            }
            "--term" => {
                pending_option = Some(value.to_string());
            }
            "--selector" => {
                pending_option = Some(value.to_string());
            }
            "--workspace" => {
                pending_option = Some(value.to_string());
            }
            "--asp-syntax-query-captures"
            | "--asp-syntax-query-node-types"
            | "--asp-syntax-query-fields"
            | "--asp-syntax-query-predicates-json" => {
                pending_option = Some(value.to_string());
            }
            "--json" => {
                json_output = true;
            }
            "--code" => {
                code_output = true;
            }
            "--help" | "-h" => {
                print_query_help();
                return Ok(Some(ExitCode::SUCCESS));
            }
            option if option.starts_with('-') => {
                return Err(format!("unsupported query catalog option: {option}"));
            }
            other => {
                positionals.push(PathBuf::from(other));
            }
        }
    }

    if let Some(option) = pending_option {
        return Err(format!("missing value for query catalog option {option}"));
    }
    if workspace_root.is_some() && !positionals.is_empty() {
        return Err(
            "query accepts project root via --workspace or positional PROJECT_ROOT, not both"
                .to_string(),
        );
    }
    if code_output && !positionals.is_empty() {
        return Err(
            "query --code does not accept a trailing PROJECT_ROOT; use --workspace PROJECT_ROOT"
                .to_string(),
        );
    }
    if positionals.len() > 1 {
        return Err("query catalog accepts at most one project root".to_string());
    }
    if catalog_id.is_some() && tree_sitter_query.is_some() {
        return Err("query accepts only one of --catalog or --treesitter-query".to_string());
    }

    let project_root = workspace_root
        .or_else(|| positionals.first().cloned())
        .unwrap_or_else(|| PathBuf::from("."));

    let (
        input,
        input_form,
        catalog_id,
        catalog_path,
        catalog_source,
        catalog_canonical,
        catalog_embedded,
        query_plan,
    ) = if let Some(catalog_id) = catalog_id {
        let catalog = rust_tree_sitter_catalog(&catalog_id)
            .ok_or_else(|| format!("unknown Rust tree-sitter query catalog: {catalog_id}"))?;
        let query_plan = asp_plan.unwrap_or_else(|| catalog.query_plan());
        (
            catalog.id.to_string(),
            "catalog-id",
            Some(catalog.id.to_string()),
            Some(catalog.path.to_string()),
            catalog.source.to_string(),
            true,
            true,
            query_plan,
        )
    } else {
        let query = tree_sitter_query
            .ok_or_else(|| "missing --catalog or --treesitter-query value".to_string())?;
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("query --treesitter-query value cannot be empty".to_string());
        }
        let query_plan = asp_plan.ok_or_else(|| {
            "tree-sitter query projection requires ASP-compiled query plan; use `asp rust query --treesitter-query ...` so ASP owns query ABI compilation".to_string()
        })?;
        (
            query.clone(),
            "s-expression",
            None,
            None,
            query,
            false,
            false,
            query_plan,
        )
    };

    let query_identity = catalog_id.as_deref().unwrap_or("inline");
    let catalog_fingerprint = syntax_catalog_fingerprint(
        query_identity,
        catalog_path.as_deref().unwrap_or("<inline>"),
        &catalog_source,
    );
    let grammar_profile_fingerprint = grammar_profile_fingerprint(
        RUST_TREE_SITTER_GRAMMAR_PROFILE_PATH,
        RUST_TREE_SITTER_GRAMMAR_PROFILE_SOURCE,
    );
    let artifact_stem = catalog_id
        .clone()
        .unwrap_or_else(|| format!("inline-{}", fingerprint_suffix(&catalog_fingerprint)));
    let request_fingerprint = format!(
        "semantic-tree-sitter-query.v1:rust:{RUST_TREE_SITTER_GRAMMAR_ID}:{query_identity}:{catalog_fingerprint}:{grammar_profile_fingerprint}",
    );
    let projection_root = project_root.clone();
    let projection_node_types = query_plan.node_types.clone();
    let projection_fields = query_plan.fields.clone();
    let projection_captures = query_plan.captures.clone();
    let projection_terms = terms.clone();
    let projection_predicates = query_plan.predicates.clone();
    let projection_selector = selector.clone();
    let projection = std::thread::Builder::new()
        .name("rs-harness-tree-sitter-query".to_string())
        .stack_size(TREE_SITTER_QUERY_WORKER_STACK_BYTES)
        .spawn(move || {
            project_tree_sitter_query(
                &projection_root,
                &projection_node_types,
                &projection_fields,
                &projection_captures,
                &projection_terms,
                &projection_predicates,
                projection_selector.as_ref(),
            )
        })
        .map_err(|error| format!("failed to start tree-sitter query worker: {error}"))?
        .join()
        .map_err(|panic| {
            if let Some(message) = panic.downcast_ref::<&str>() {
                format!("tree-sitter query worker panicked: {message}")
            } else if let Some(message) = panic.downcast_ref::<String>() {
                format!("tree-sitter query worker panicked: {message}")
            } else {
                "tree-sitter query worker panicked".to_string()
            }
        })??;

    if json_output {
        let predicates = query_plan
            .predicates
            .iter()
            .map(format_query_predicate)
            .collect::<Vec<_>>();
        let mut query_fields = serde_json::json!({
            "captures": &query_plan.captures,
            "nodeTypes": &query_plan.node_types,
            "fields": &query_plan.fields,
            "predicates": predicates,
            "unsupportedPredicates": &projection.unsupported_predicates,
            "catalogCanonical": catalog_canonical,
            "catalogEmbedded": catalog_embedded,
            "compilerBoundary": "asp-tree-sitter-runtime",
            "providerRuntimeCompiled": false,
            "codeOutput": code_output,
            "terms": terms
        });
        if let Some(selector) = selector.as_ref() {
            query_fields["selector"] = serde_json::json!(selector.display());
        }
        let mut query = serde_json::json!({
            "input": input,
            "inputForm": input_form,
            "dialect": "tree-sitter-query",
            "grammarProfilePath": RUST_TREE_SITTER_GRAMMAR_PROFILE_PATH,
            "compiledSource": catalog_source,
            "fields": query_fields
        });
        if let Some(catalog_id) = catalog_id.as_deref() {
            query["catalogId"] = serde_json::json!(catalog_id);
        }
        if let Some(catalog_path) = catalog_path.as_deref() {
            query["catalogPath"] = serde_json::json!(catalog_path);
        }
        let matches = syntax_query_matches_json(&projection.rows);
        let native_fact_refs = syntax_query_native_fact_refs(&projection.rows);

        let packet = serde_json::json!({
            "schemaId": "agent.semantic-protocols.semantic-tree-sitter-query",
            "schemaVersion": "1",
            "protocolId": "agent.semantic-protocols.semantic-language",
            "protocolVersion": "1",
            "languageId": "rust",
            "providerId": "rs-harness",
            "method": "query",
            "projectRoot": project_root.display().to_string(),
            "grammarId": RUST_TREE_SITTER_GRAMMAR_ID,
            "grammarProfileVersion": RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION,
            "sourceAuthority": "native-parser-adapter",
            "adapterMode": "native-projection",
            "compatibilityLevel": "native-only",
            "query": query,
            "matches": matches,
            "nativeFactRefs": native_fact_refs,
            "truncated": projection.truncated,
            "cache": {
                "cacheStatus": "miss",
                "requestFingerprint": request_fingerprint,
                "generationId": format!("rust-tree-sitter-query:{artifact_stem}:{RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION}"),
                "artifactId": format!("semantic-tree-sitter-query/{artifact_stem}.json"),
                "artifactKind": "semantic-tree-sitter-query",
                "catalogFingerprint": catalog_fingerprint,
                "grammarProfileFingerprint": grammar_profile_fingerprint,
                "rawSourceStored": false
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&packet).map_err(|error| format!(
                "failed to serialize semantic tree-sitter query packet: {error}"
            ))?
        );
    } else {
        if code_output {
            if projection.total_matches > 1 {
                return Err(format!(
                    "query --code matched {} items; add an exact --selector, a unique predicate, or narrow the tree-sitter query before requesting pure code",
                    projection.total_matches
                ));
            }
            print_tree_sitter_query_code(&projection.rows);
            return Ok(Some(ExitCode::SUCCESS));
        }
        if projection.rows.is_empty() {
            let term_field = if terms.is_empty() {
                String::new()
            } else {
                format!(" terms={}", terms.join(","))
            };
            let captures_display = if query_plan.captures.is_empty() {
                "none".to_string()
            } else {
                query_plan.captures.join(",")
            };
            println!(
                "|syntax-query inputForm={} input={} grammar={} grammarProfile={} dialect=tree-sitter-query mode=native-parser-projection matchStatus={} match={} rows={} truncated={} captureCount={} captures={}{} catalogCanonical={} catalogEmbedded={} sourceAuthority=native-parser compilerBoundary=asp-tree-sitter-runtime providerRuntimeCompiled=false",
                input_form,
                query_identity,
                RUST_TREE_SITTER_GRAMMAR_ID,
                RUST_TREE_SITTER_GRAMMAR_PROFILE_VERSION,
                projection.match_status(),
                projection.total_matches,
                projection.rows.len(),
                projection.truncated,
                query_plan.captures.len(),
                captures_display,
                term_field,
                catalog_canonical,
                catalog_embedded
            );
            if !projection.unsupported_nodes.is_empty() {
                println!(
                    "|syntax-query-unsupported nodes={} supported={}",
                    projection.unsupported_nodes.join(","),
                    SUPPORTED_TREE_SITTER_QUERY_NODES.join(",")
                );
            }
            if !projection.unsupported_predicates.is_empty() {
                println!(
                    "|syntax-query-unsupported predicates={} supported=predicate:eq,predicate:any-of,predicate:match,predicate:not-eq,predicate:not-match",
                    projection.unsupported_predicates.join(",")
                );
            }
        }
        if !projection.rows.is_empty() {
            print_tree_sitter_query_graph(
                &project_root,
                &query_plan,
                &projection.rows,
                projection.total_matches,
                projection.truncated,
            );
        }
    }

    Ok(Some(ExitCode::SUCCESS))
}

fn print_tree_sitter_query_code(rows: &[SyntaxQueryRow]) {
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("{}", row.item_code);
    }
}

fn print_tree_sitter_query_graph(
    project_root: &std::path::Path,
    query_plan: &RustSyntaxQueryPlan,
    rows: &[SyntaxQueryRow],
    total_matches: usize,
    truncated: bool,
) {
    let display_len = rows.len().min(MAX_SYNTAX_QUERY_GRAPH_ROWS);
    let display_rows = &rows[..display_len];
    let overflow = total_matches > display_len;
    let pattern = syntax_query_pattern_label(query_plan);
    let capture = query_plan
        .captures
        .first()
        .map_or("item.name", String::as_str);
    println!(
        "[query-treesitter] root={} lang=rust pattern={} capture={} alg=syntax-capture-frontier",
        project_root.display(),
        pattern,
        capture
    );
    println!(
        "legend: aliases ID:kind; node ID=kind:role(value)!next; ts=node/field; frontier ID.next"
    );
    println!("aliases=G:query,Q:tsquery,C:capture,I:item,O:owner");
    println!();
    println!("Q=tsquery:pattern({pattern})!query");
    for (index, row) in display_rows.iter().enumerate() {
        let capture_id = graph_id("C", index);
        let item_id = graph_id("I", index);
        println!(
            "{capture_id}=capture:{}({})@{}!code ts={}",
            row.capture,
            compact_graph_value(&row.capture_text),
            syntax_query_capture_locator(row),
            syntax_query_capture_ts(query_plan, row)
        );
        println!(
            "{item_id}=item:{}({})@{}!code ts={}",
            syntax_query_item_kind(row),
            compact_graph_value(&row.name),
            syntax_query_range_locator(row),
            row.node
        );
    }
    println!();
    println!("G>{{Q:selects}}");
    println!("Q>{{{}}}", graph_edges("C", "captures", display_len));
    for index in 0..display_len {
        println!(
            "{}>{{{}:enclosing-item}}",
            graph_id("C", index),
            graph_id("I", index)
        );
    }
    println!();
    if truncated || overflow {
        println!(
            "matches={} shown={} omitted={}",
            total_matches,
            display_len,
            total_matches.saturating_sub(display_len)
        );
        println!("omit=code,full-node-list,overflow-captures");
    } else {
        println!("omit=code,full-node-list,capture-text");
    }
    println!("rank={}", graph_id_list("I", display_len));
    println!("frontier={}", graph_frontier_list("I", display_len));
    println!("avoid=broad-code-output,raw-read");
}

fn syntax_query_range_locator(row: &SyntaxQueryRow) -> String {
    syntax_query_compact_locator(&row.path, row.item_start_line, row.item_end_line)
}

fn syntax_query_capture_locator(row: &SyntaxQueryRow) -> String {
    syntax_query_compact_locator(&row.path, row.start_line, row.end_line)
}

fn syntax_query_compact_locator(path: &str, start_line: usize, end_line: usize) -> String {
    let start_line = start_line.max(1);
    let end_line = end_line.max(start_line);
    if start_line == end_line {
        format!("{path}:{start_line}")
    } else {
        format!("{path}:{start_line}:{end_line}")
    }
}

fn syntax_query_pattern_label(query_plan: &RustSyntaxQueryPlan) -> String {
    let node = query_plan
        .node_types
        .iter()
        .find(|node| node.ends_with("_item"))
        .or_else(|| query_plan.node_types.first())
        .map_or("node", String::as_str);
    query_plan
        .fields
        .first()
        .map_or_else(|| node.to_string(), |field| format!("{node}/{field}"))
}

fn syntax_query_capture_ts(_query_plan: &RustSyntaxQueryPlan, row: &SyntaxQueryRow) -> String {
    format!("{}/{}", row.capture_node, row.capture_field)
}

fn syntax_query_item_kind(row: &SyntaxQueryRow) -> &'static str {
    match row.node {
        "const_item" => "const",
        "call_expression" => "call",
        "enum_item" => "enum",
        "function_item" => "fn",
        "impl_item" => "impl",
        "mod_item" => "mod",
        "static_item" => "static",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "type_item" => "type",
        "use_declaration" => "use",
        _ => "item",
    }
}

fn compact_graph_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("_")
}

fn graph_id(prefix: &str, index: usize) -> String {
    if index == 0 {
        prefix.to_string()
    } else {
        format!("{prefix}{}", index + 1)
    }
}

fn graph_id_list(prefix: &str, len: usize) -> String {
    (0..len)
        .map(|index| graph_id(prefix, index))
        .collect::<Vec<_>>()
        .join(",")
}

fn graph_frontier_list(prefix: &str, len: usize) -> String {
    (0..len)
        .map(|index| format!("{}.code", graph_id(prefix, index)))
        .collect::<Vec<_>>()
        .join(",")
}

fn graph_edges(prefix: &str, edge: &str, len: usize) -> String {
    (0..len)
        .map(|index| format!("{}:{edge}", graph_id(prefix, index)))
        .collect::<Vec<_>>()
        .join(",")
}

fn rust_tree_sitter_catalog(catalog_id: &str) -> Option<RustTreeSitterCatalog> {
    match catalog_id {
        "declarations" => Some(RustTreeSitterCatalog {
            id: "declarations",
            path: "tree-sitter/tree-sitter-rust/queries/declarations.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/declarations.scm"),
            captures: &[
                "constant.definition",
                "constant.name",
                "constant.type",
                "function.definition",
                "function.modifier",
                "function.name",
                "function.return_type",
                "function.type_parameters",
                "impl.definition",
                "impl.target",
                "impl.trait",
                "impl.type_parameters",
                "item.attribute",
                "item.visibility",
                "module.definition",
                "module.name",
                "trait.bounds",
                "trait.definition",
                "trait.name",
                "trait.type_parameters",
                "type.aliased_type",
                "type.definition",
                "type.name",
                "type.type_parameters",
            ],
            node_types: &[
                "attribute_item",
                "const_item",
                "enum_item",
                "function_item",
                "impl_item",
                "mod_item",
                "static_item",
                "struct_item",
                "trait_item",
                "type_item",
                "union_item",
            ],
            fields: &[
                "body",
                "bounds",
                "declarator",
                "name",
                "return_type",
                "trait",
                "type",
                "type_parameters",
                "value",
            ],
        }),
        "imports" => Some(RustTreeSitterCatalog {
            id: "imports",
            path: "tree-sitter/tree-sitter-rust/queries/imports.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/imports.scm"),
            captures: &[
                "import.alias",
                "import.declaration",
                "import.path",
                "import.visibility",
            ],
            node_types: &["extern_crate_declaration", "use_declaration"],
            fields: &["alias", "crate", "name"],
        }),
        "calls" => Some(RustTreeSitterCatalog {
            id: "calls",
            path: "tree-sitter/tree-sitter-rust/queries/calls.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/calls.scm"),
            captures: &["call.expression", "call.method", "call.target"],
            node_types: &[
                "call_expression",
                "field_expression",
                "identifier",
                "scoped_identifier",
            ],
            fields: &["function"],
        }),
        "macros" => Some(RustTreeSitterCatalog {
            id: "macros",
            path: "tree-sitter/tree-sitter-rust/queries/macros.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/macros.scm"),
            captures: &["macro.arguments", "macro.invocation", "macro.name"],
            node_types: &["macro_invocation", "token_tree"],
            fields: &["macro"],
        }),
        "cfg" => Some(RustTreeSitterCatalog {
            id: "cfg",
            path: "tree-sitter/tree-sitter-rust/queries/cfg.scm",
            source: include_str!("../../tree-sitter/tree-sitter-rust/queries/cfg.scm"),
            captures: &[
                "attribute.arguments",
                "attribute.body",
                "attribute.item",
                "attribute.name",
                "attribute.value",
            ],
            node_types: &[
                "attribute",
                "attribute_item",
                "identifier",
                "meta_item",
                "string_literal",
            ],
            fields: &[],
        }),
        _ => None,
    }
}

impl RustTreeSitterCatalog {
    fn query_plan(&self) -> RustSyntaxQueryPlan {
        RustSyntaxQueryPlan {
            captures: self
                .captures
                .iter()
                .map(|capture| (*capture).to_string())
                .collect(),
            node_types: self
                .node_types
                .iter()
                .map(|node_type| (*node_type).to_string())
                .collect(),
            fields: self
                .fields
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
            predicates: Vec::new(),
        }
    }
}

fn split_internal_plan_list(value: &str) -> Vec<String> {
    let mut values = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn parse_internal_query_predicates_json(value: &str) -> Result<Vec<SyntaxQueryPredicate>, String> {
    let predicates = serde_json::from_str::<Vec<SyntaxQueryPredicate>>(value)
        .map_err(|error| format!("invalid ASP syntax query predicate JSON: {error}"))?;
    for predicate in &predicates {
        if predicate.capture.trim().is_empty() {
            return Err("ASP syntax query predicate capture cannot be empty".to_string());
        }
        if predicate.values.is_empty() {
            return Err(format!(
                "ASP syntax query predicate {}:{} has no values",
                predicate_op_label(&predicate.op),
                predicate.capture
            ));
        }
        for operand in &predicate.values {
            match operand {
                SyntaxQueryPredicateValue::String(value)
                | SyntaxQueryPredicateValue::Capture(value)
                    if value.trim().is_empty() =>
                {
                    return Err(format!(
                        "ASP syntax query predicate {}:{} has an empty operand",
                        predicate_op_label(&predicate.op),
                        predicate.capture
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(predicates)
}

fn format_query_predicate(predicate: &SyntaxQueryPredicate) -> serde_json::Value {
    serde_json::json!({
        "op": predicate_op_label(&predicate.op),
        "capture": predicate.capture.as_str(),
        "values": predicate.values.iter().map(|value| match value {
            SyntaxQueryPredicateValue::String(value) => serde_json::json!({
                "kind": "string",
                "value": value.as_str()
            }),
            SyntaxQueryPredicateValue::Capture(value) => serde_json::json!({
                "kind": "capture",
                "value": value.as_str()
            }),
        }).collect::<Vec<_>>()
    })
}

fn predicate_op_label(op: &SyntaxQueryPredicateOp) -> &'static str {
    match op {
        SyntaxQueryPredicateOp::Eq => "eq",
        SyntaxQueryPredicateOp::AnyEq => "any-eq",
        SyntaxQueryPredicateOp::AnyOf => "any-of",
        SyntaxQueryPredicateOp::Match => "match",
        SyntaxQueryPredicateOp::AnyMatch => "any-match",
        SyntaxQueryPredicateOp::NotEq => "not-eq",
        SyntaxQueryPredicateOp::NotMatch => "not-match",
    }
}

fn syntax_catalog_fingerprint(id: &str, path: &str, source: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    path.hash(&mut hasher);
    source.hash(&mut hasher);
    format!("syntax-catalog:{:016x}", hasher.finish())
}

fn grammar_profile_fingerprint(path: &str, source: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    source.hash(&mut hasher);
    format!("grammar-profile:{:016x}", hasher.finish())
}

fn fingerprint_suffix(fingerprint: &str) -> &str {
    fingerprint
        .rsplit_once(':')
        .map_or(fingerprint, |(_, suffix)| suffix)
}
