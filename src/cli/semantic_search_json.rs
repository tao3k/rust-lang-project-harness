//! Shared semantic-search JSON envelope for CLI search output.

use std::path::Path;

use serde_json::{Map, Value, json};

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
    let package_name = options
        .package
        .clone()
        .or_else(|| header_package(&packet).map(ToOwned::to_owned));
    if let Some(package_name) = package_name {
        packet["packageName"] = json!(package_name);
    }
    packet
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

fn parse_fields<'a>(tokens: impl IntoIterator<Item = &'a str>) -> Map<String, Value> {
    let mut fields = Map::new();
    for token in tokens {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        let key = json_field_key(key);
        fields.insert(key.clone(), parse_field_value_for_key(&key, value));
    }
    fields
}

fn json_field_key(key: &str) -> String {
    let mut words = key.split('_');
    let Some(first) = words.next() else {
        return key.to_string();
    };
    let mut normalized = first.to_string();
    for word in words {
        let mut chars = word.chars();
        let Some(first_char) = chars.next() else {
            continue;
        };
        normalized.extend(first_char.to_uppercase());
        normalized.push_str(chars.as_str());
    }
    normalized
}

fn parse_field_value_for_key(key: &str, value: &str) -> Value {
    if matches!(
        key,
        "requestedVersion" | "currentWorkspaceVersion" | "workspaceResolvedVersion"
    ) {
        return Value::String(value.to_string());
    }
    parse_field_value(value)
}

fn parse_field_value(value: &str) -> Value {
    if value.contains(',') {
        return Value::Array(value.split(',').map(parse_field_value).collect());
    }
    match value {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => value
            .parse::<i64>()
            .map(|number| json!(number))
            .or_else(|_| value.parse::<f64>().map(|number| json!(number)))
            .unwrap_or_else(|_| Value::String(value.to_string())),
    }
}

fn next_field(fields: &Map<String, Value>) -> Option<String> {
    fields.get("next").map(|value| match value {
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(","),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    })
}

fn parse_next_actions(fragment: String, owner_path: Option<&str>) -> Vec<Value> {
    fragment
        .split(',')
        .map(str::trim)
        .filter(|fragment| !fragment.is_empty() && *fragment != "-")
        .map(|fragment| parse_next_action(fragment, owner_path))
        .collect()
}

fn parse_next_action(fragment: &str, owner_path: Option<&str>) -> Value {
    let (body, scope) = fragment
        .split_once("(scope=")
        .map(|(body, scope)| (body, Some(scope.trim_end_matches(')'))))
        .unwrap_or((fragment, None));
    let (kind, target) = body.split_once(':').unwrap_or((body, "."));
    let mut action = json!({
        "kind": kind,
        "target": target,
    });
    if let Some(scope) = scope {
        action["scope"] = json!(scope);
    }
    if let Some(owner_path) = owner_path {
        action["ownerPath"] = json!(owner_path);
    }
    action
}

fn parse_edge_kind(token: &str) -> (String, Option<String>) {
    let edge = token.trim_start_matches('-').trim_end_matches("->");
    edge.split_once(':')
        .map(|(kind, label)| (kind.to_string(), Some(label.to_string())))
        .unwrap_or_else(|| (edge.to_string(), None))
}

fn input_detection_from_header(fields: &Map<String, Value>) -> Option<Value> {
    let source = string_field(fields, "src")?;
    let source = match source.as_str() {
        "paths" => "path-list",
        other => other,
    };
    Some(json!({
        "source": source,
        "lineCount": usize_field(fields, "in").unwrap_or(0),
        "byteCount": 0,
    }))
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

fn header_package(packet: &Value) -> Option<&str> {
    packet["header"]["fields"]["package"].as_str()
}

fn string_field(fields: &Map<String, Value>, key: &str) -> Option<String> {
    fields.get(key).and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(","),
        ),
        _ => None,
    })
}

fn bool_field(fields: &Map<String, Value>, key: &str) -> Option<bool> {
    fields.get(key).and_then(Value::as_bool)
}

fn usize_field(fields: &Map<String, Value>, key: &str) -> Option<usize> {
    fields
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn insert_if_some(fields: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn insert_if_usize(fields: &mut Map<String, Value>, key: &str, value: Option<usize>) {
    if let Some(value) = value {
        fields.insert(key.to_string(), json!(value));
    }
}

fn location_from_node(node: &str) -> Value {
    location(node.strip_prefix("O:").unwrap_or(node))
}

fn location(path: &str) -> Value {
    json!({ "path": path })
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
