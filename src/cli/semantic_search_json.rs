//! Shared semantic-search JSON envelope for CLI search output.

use std::path::Path;

use serde_json::{Map, Value, json};

use super::semantic_search_json_fields::{
    bool_field, display_path, header_package, input_detection_from_header, insert_if_some,
    insert_if_usize, location, location_from_node, next_field, parse_edge_kind, parse_fields,
    parse_next_actions, string_field,
};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-search-packet";
const SCHEMA_VERSION: &str = "1";

pub(super) struct SemanticSearchJsonOptions {
    pub(super) view: String,
    pub(super) query: Option<String>,
    pub(super) command_label: String,
    pub(super) trace: bool,
    pub(super) explain: bool,
    pub(super) output_view: Option<String>,
    pub(super) depth: Option<usize>,
    pub(super) dir: Option<String>,
    pub(super) edges: Vec<String>,
    pub(super) per_owner: Option<usize>,
    pub(super) seeds: Option<usize>,
    pub(super) owners: Option<usize>,
    pub(super) hits: Option<usize>,
    pub(super) package: Option<String>,
    pub(super) owner: Option<String>,
    pub(super) dependency: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) lines: bool,
    pub(super) pipes: Vec<String>,
    pub(super) query_set: Vec<String>,
}

pub(super) fn render_search_json(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    rendered: &str,
) -> Result<String, String> {
    let packet = build_packet(project_root, options, rendered);
    serde_json::to_string(&packet).map_err(|error| format!("failed to render search JSON: {error}"))
}

fn build_packet(project_root: &Path, options: &SemanticSearchJsonOptions, rendered: &str) -> Value {
    let (header_kind, mut header_fields) = parse_header(rendered, options);
    enrich_header_fields(&mut header_fields, options);

    let mut packages = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut owners = Vec::new();
    let mut items = Vec::new();
    let mut hits = Vec::new();
    let mut findings = Vec::new();
    let mut next_actions = Vec::new();
    let mut notes = Vec::new();
    let mut current_owner = None::<String>;

    for line in rendered.lines().filter_map(|line| line.strip_prefix('|')) {
        let mut tokens = line.split_whitespace();
        let Some(tag) = tokens.next() else {
            continue;
        };
        let remaining = tokens.collect::<Vec<_>>();
        match tag {
            "node" => push_node(&remaining, &mut nodes, &mut next_actions),
            "package" => push_package(&remaining, &mut packages),
            "dep" => push_dependency_node(&remaining, &mut nodes),
            "owner" => {
                if let Some(owner_path) = push_owner(&remaining, &mut owners, &mut next_actions) {
                    current_owner = Some(owner_path);
                }
            }
            "item" => push_item(
                &remaining,
                current_owner.as_deref(),
                &mut items,
                &mut next_actions,
            ),
            "edge" => push_edge(&remaining, &mut edges),
            "find" => push_finding(&remaining, &mut findings),
            "next" => push_next_actions(remaining.join(" "), None, &mut next_actions),
            "def" | "call" | "api" => push_hit(tag, &remaining, &mut hits),
            "external-type" => push_hit(tag, &remaining, &mut hits),
            "api-candidate" => push_api_candidate(&remaining, &mut hits),
            "test" => push_test_node(&remaining, &mut nodes, &mut next_actions),
            "note" => push_note(&remaining, &mut notes),
            "feat" | "feature" | "cfg" | "target" | "pat" => {
                push_fact_note(tag, &remaining, &mut notes)
            }
            _ => push_raw_note(tag, &remaining, &mut notes),
        }
    }

    let input_detection = if options.view == "ingest" {
        input_detection_from_header(&header_fields)
    } else {
        None
    };
    if let Some(input_detection) = input_detection {
        return packet_with_input_detection(
            project_root,
            options,
            header_kind,
            header_fields,
            input_detection,
            PacketCollections {
                packages,
                nodes,
                edges,
                owners,
                items,
                hits,
                findings,
                next_actions,
                notes,
            },
        );
    }

    base_packet(
        project_root,
        options,
        header_kind,
        header_fields,
        PacketCollections {
            packages,
            nodes,
            edges,
            owners,
            items,
            hits,
            findings,
            next_actions,
            notes,
        },
    )
}

struct PacketCollections {
    packages: Vec<Value>,
    nodes: Vec<Value>,
    edges: Vec<Value>,
    owners: Vec<Value>,
    items: Vec<Value>,
    hits: Vec<Value>,
    findings: Vec<Value>,
    next_actions: Vec<Value>,
    notes: Vec<Value>,
}

fn base_packet(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    header_kind: String,
    header_fields: Map<String, Value>,
    collections: PacketCollections,
) -> Value {
    let mut packet = json!({
        "schemaId": SCHEMA_ID,
        "schemaVersion": SCHEMA_VERSION,
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "binary": "rs-harness",
        "namespace": "agent.semantic-protocols.semantic-language",
        "method": format!("search/{}", schema_view(options)),
        "projectRoot": display_path(project_root),
        "view": schema_view(options),
        "renderMode": render_mode(options),
        "header": {
            "kind": header_kind,
            "fields": header_fields,
        },
        "packages": collections.packages,
        "nodes": collections.nodes,
        "edges": collections.edges,
        "owners": collections.owners,
        "items": collections.items,
        "hits": collections.hits,
        "findings": collections.findings,
        "nextActions": collections.next_actions,
        "notes": collections.notes,
    });
    if let Some(query) = options.query.as_deref() {
        packet["query"] = json!(query);
    }
    if !options.query_set.is_empty() {
        packet["querySet"] = json!(
            options
                .query_set
                .iter()
                .map(|term| {
                    json!({
                        "value": term,
                        "kind": query_set_kind(options),
                        "selector": "exact"
                    })
                })
                .collect::<Vec<_>>()
        );
        packet["queryComposition"] = json!({
            "mode": "query-set",
            "view": schema_view(options),
            "selector": "exact-set",
            "merge": [
                "packages",
                "nodes",
                "edges",
                "owners",
                "items",
                "hits",
                "findings",
                "nextActions",
                "notes"
            ],
            "fields": {
                "count": options.query_set.len()
            }
        });
    }
    let package_name = options
        .package
        .clone()
        .or_else(|| header_package(&packet).map(ToOwned::to_owned));
    if let Some(package_name) = package_name {
        packet["packageName"] = json!(package_name);
    }
    packet
}

fn query_set_kind(options: &SemanticSearchJsonOptions) -> &'static str {
    match options.view.as_str() {
        "dependency" | "deps" => "dependency",
        "owner" | "tests" => "owner",
        "features" => "feature",
        "cfg" => "cfg",
        "api" | "docs" | "docs-use" | "public-external-types" => "api",
        "symbol" | "callsite" | "import" => "symbol",
        "text" => "text",
        _ => "custom",
    }
}

fn packet_with_input_detection(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    header_kind: String,
    header_fields: Map<String, Value>,
    input_detection: Value,
    collections: PacketCollections,
) -> Value {
    let mut packet = base_packet(
        project_root,
        options,
        header_kind,
        header_fields,
        collections,
    );
    packet["inputDetection"] = input_detection;
    packet
}

fn parse_header(
    rendered: &str,
    options: &SemanticSearchJsonOptions,
) -> (String, Map<String, Value>) {
    let fallback = format!("search-{}", options.command_label.replace(' ', "-"));
    let Some(line) = rendered.lines().find(|line| line.starts_with('[')) else {
        return (fallback, Map::new());
    };
    let Some((kind, tail)) = line.strip_prefix('[').and_then(|line| line.split_once(']')) else {
        return (fallback, Map::new());
    };
    (kind.to_string(), parse_fields(tail.split_whitespace()))
}

fn enrich_header_fields(fields: &mut Map<String, Value>, options: &SemanticSearchJsonOptions) {
    insert_if_some(fields, "packageSelector", options.package.as_deref());
    insert_if_some(fields, "ownerSelector", options.owner.as_deref());
    insert_if_some(fields, "dependencySelector", options.dependency.as_deref());
    insert_if_some(fields, "scope", options.scope.as_deref());
    if !options.pipes.is_empty() {
        fields.insert("pipes".to_string(), json!(options.pipes));
    }
    if options.trace {
        fields.insert("trace".to_string(), Value::Bool(true));
    }
    if options.explain {
        fields.insert("explain".to_string(), Value::Bool(true));
    }
    if options.lines {
        fields.insert("lines".to_string(), Value::Bool(true));
    }
    insert_if_some(fields, "dir", options.dir.as_deref());
    insert_if_usize(fields, "depth", options.depth);
    insert_if_usize(fields, "perOwner", options.per_owner);
    insert_if_usize(fields, "seeds", options.seeds);
    insert_if_usize(fields, "owners", options.owners);
    insert_if_usize(fields, "hits", options.hits);
    if !options.edges.is_empty() {
        fields.insert("edge".to_string(), json!(options.edges));
    }
}

fn push_package(tokens: &[&str], packages: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    packages.push(json!({
        "id": id,
        "fields": parse_fields(tokens.iter().skip(1).copied()),
    }));
}

fn push_node(tokens: &[&str], nodes: &mut Vec<Value>, next_actions: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, None))
            .unwrap_or_default(),
    );
    nodes.push(json!({
        "id": id,
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "node".to_string()),
        "fields": fields,
    }));
}

fn push_dependency_node(tokens: &[&str], nodes: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    nodes.push(json!({
        "id": format!("D:{id}"),
        "kind": "dependency",
        "fields": parse_fields(tokens.iter().skip(1).copied()),
    }));
}

fn push_owner(
    tokens: &[&str],
    owners: &mut Vec<Value>,
    next_actions: &mut Vec<Value>,
) -> Option<String> {
    let path = tokens.first()?.to_string();
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let role = string_field(&fields, "role").unwrap_or_else(|| "source".to_string());
    let public = bool_field(&fields, "public")
        .or_else(|| bool_field(&fields, "pub"))
        .unwrap_or_else(|| role.contains("public"));
    let owner_next = next_field(&fields)
        .map(|next| parse_next_actions(next, Some(path.as_str())))
        .unwrap_or_default();
    next_actions.extend(owner_next.iter().cloned());
    owners.push(json!({
        "path": path,
        "role": role,
        "public": public,
        "nextActions": owner_next,
        "fields": fields,
    }));
    Some(path)
}

fn push_item(
    tokens: &[&str],
    current_owner: Option<&str>,
    items: &mut Vec<Value>,
    next_actions: &mut Vec<Value>,
) {
    let Some(name) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = current_owner.unwrap_or("-");
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, Some(owner_path)))
            .unwrap_or_default(),
    );
    items.push(json!({
        "name": name,
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "item".to_string()),
        "ownerPath": owner_path,
        "location": location(owner_path),
        "fields": fields,
    }));
}

fn push_edge(tokens: &[&str], edges: &mut Vec<Value>) {
    if tokens.len() < 3 {
        return;
    }
    let (kind, label) = parse_edge_kind(tokens[1]);
    let mut edge = json!({
        "from": tokens[0],
        "kind": kind,
        "to": tokens[2],
    });
    if let Some(label) = label {
        edge["label"] = json!(label);
    }
    edges.push(edge);
}

fn push_finding(tokens: &[&str], findings: &mut Vec<Value>) {
    let Some(rule_id) = tokens.first() else {
        return;
    };
    let count = tokens
        .iter()
        .find_map(|token| token.strip_prefix('x'))
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(1);
    let fields = parse_fields(tokens.iter().skip(1).copied().filter(|token| {
        token
            .strip_prefix('x')
            .is_none_or(|count| count.parse::<usize>().is_err())
    }));
    findings.push(json!({
        "ruleId": rule_id,
        "severity": string_field(&fields, "severity").unwrap_or_else(|| "warning".to_string()),
        "count": count,
        "location": location_from_node(string_field(&fields, "at").as_deref().unwrap_or("memory")),
        "fields": fields,
    }));
}

fn push_hit(kind: &str, tokens: &[&str], hits: &mut Vec<Value>) {
    let Some(path) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let mut hit = json!({
        "kind": kind,
        "ownerPath": path,
        "location": location(path),
        "score": 1.0,
        "reason": kind,
        "fields": fields,
    });
    if let Some(symbol) = string_field(&fields, "name") {
        hit["symbol"] = json!(symbol);
    }
    hits.push(hit);
}

fn push_api_candidate(tokens: &[&str], hits: &mut Vec<Value>) {
    let Some(symbol) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = string_field(&fields, "owner").unwrap_or_else(|| "-".to_string());
    hits.push(json!({
        "kind": "api-candidate",
        "ownerPath": owner_path,
        "symbol": symbol,
        "location": location(string_field(&fields, "owner").as_deref().unwrap_or("-")),
        "score": 1.0,
        "reason": string_field(&fields, "reason").unwrap_or_else(|| "public-item".to_string()),
        "fields": fields,
    }));
}

fn push_test_node(tokens: &[&str], nodes: &mut Vec<Value>, next_actions: &mut Vec<Value>) {
    let Some(path) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, Some(path)))
            .unwrap_or_default(),
    );
    nodes.push(json!({
        "id": format!("T:{path}"),
        "kind": "test",
        "path": path,
        "fields": fields,
    }));
}

fn push_note(tokens: &[&str], notes: &mut Vec<Value>) {
    let fields = parse_fields(tokens.iter().copied());
    notes.push(json!({
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "note".to_string()),
        "message": string_field(&fields, "message").unwrap_or_else(|| tokens.join(" ")),
        "fields": fields,
    }));
}

fn push_fact_note(tag: &str, tokens: &[&str], notes: &mut Vec<Value>) {
    notes.push(json!({
        "kind": tag,
        "message": tokens.join(" "),
        "fields": parse_fields(tokens.iter().skip(1).copied()),
    }));
}

fn push_raw_note(tag: &str, tokens: &[&str], notes: &mut Vec<Value>) {
    notes.push(json!({
        "kind": "line",
        "message": format!("{tag} {}", tokens.join(" ")).trim().to_string(),
    }));
}

fn push_next_actions(fragment: String, owner_path: Option<&str>, next_actions: &mut Vec<Value>) {
    next_actions.extend(parse_next_actions(fragment, owner_path));
}

fn schema_view(options: &SemanticSearchJsonOptions) -> &str {
    options.view.as_str()
}

fn render_mode(options: &SemanticSearchJsonOptions) -> &str {
    if let Some(output_view) = options.output_view.as_deref() {
        return output_view;
    }
    match options.view.as_str() {
        "symbol"
        | "callsite"
        | "text"
        | "pattern"
        | "docs"
        | "docs-use"
        | "api"
        | "public-external-types"
        | "deps"
        | "cfg" => "hits",
        _ => "graph",
    }
}
