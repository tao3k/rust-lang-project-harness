//! JSON renderer for provider-native parser query packets.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::parser::native_syntax::projection_code;

use super::semantic_query_projection::{
    projection_node_classification, projection_semantic_responsibilities,
    replace_item_patch_safety, string_field_value,
};
use super::semantic_search_json_fields::{display_path, parse_fields, string_field};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-query-packet";
const SCHEMA_VERSION: &str = "1";
const MAX_QUERY_PACKET_MATCHES: usize = 4;
const MAX_QUERY_PACKET_PROJECTION_NODES: usize = 24;
const MAX_QUERY_PACKET_EXPAND_ACTIONS: usize = 8;

pub(super) struct SemanticQueryJsonOptions {
    pub(super) query: String,
    pub(super) item_names_only: bool,
}

pub(super) fn render_query_json(
    project_root: &Path,
    options: &SemanticQueryJsonOptions,
    rendered: &str,
) -> Result<String, String> {
    let packet = build_packet(project_root, options, rendered);
    serde_json::to_string(&packet).map_err(|error| format!("failed to render query JSON: {error}"))
}

fn build_packet(project_root: &Path, options: &SemanticQueryJsonOptions, rendered: &str) -> Value {
    let mut owner_path = None::<String>;
    let mut query_coverage = Vec::new();
    let mut candidate_items = Vec::new();
    let mut matches = Vec::<Value>::new();
    let mut match_mode = "unknown".to_string();
    let mut output_mode = if options.item_names_only {
        "names"
    } else {
        "code"
    }
    .to_string();

    for line in rendered.lines().filter(|line| line.starts_with('|')) {
        if let Some(rest) = line.strip_prefix("|owner ") {
            owner_path = rest.split_whitespace().next().map(ToOwned::to_owned);
        } else if let Some(rest) = line.strip_prefix("|query ") {
            let fields = parse_fields(rest.split_whitespace());
            if let Some(mode) = string_field(&fields, "match") {
                match_mode = match mode.as_str() {
                    "exact" | "fallback-contains" => mode,
                    _ => "unknown".to_string(),
                };
            }
            if let Some(output) = string_field(&fields, "output") {
                output_mode = output;
            }
            let (coverage, candidates) = query_coverage_from_fields(&options.query, &fields);
            query_coverage.push(coverage);
            candidate_items.extend(candidates);
        } else if let Some(rest) = line.strip_prefix("|item ") {
            if let Some(item) = item_match_from_line(rest, owner_path.as_deref()) {
                matches.push(item);
            }
        } else if let Some(rest) = line.strip_prefix("|code ") {
            attach_code_to_last_match(rest, &mut matches);
        }
    }

    let query_terms = options
        .query
        .split('|')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    let match_count = matches.len();
    let matches_truncated = match_count > MAX_QUERY_PACKET_MATCHES;
    if matches_truncated {
        matches.truncate(MAX_QUERY_PACKET_MATCHES);
    }
    let mut packet = json!({
            "schemaId": SCHEMA_ID,
            "schemaVersion": SCHEMA_VERSION,
            "protocolId": "agent.semantic-protocols.semantic-language",
            "protocolVersion": "1",
            "languageId": "rust",
            "providerId": "rs-harness",
            "binary": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness",
            "method": "query/owner-items",
            "projectRoot": display_path(project_root),
            "query": options.query,
            "queryTerms": query_terms,
            "matchMode": match_mode,
            "outputMode": output_mode,
            "patchSafety": {
                "level": "read-safe",
                "reason": "compact query packet is not a mutation authority",
                "nextAction": "query --from-hook direct-source-read",
            },
    "matches": matches,
    "truncated": matches_truncated,
    });
    if matches_truncated {
        packet["matchCount"] = json!(match_count);
        packet["matchLimit"] = json!(MAX_QUERY_PACKET_MATCHES);
        packet["matchesTruncated"] = json!(true);
    }
    if let Some(owner_path) = owner_path {
        packet["ownerPath"] = json!(owner_path);
    }
    if !query_coverage.is_empty() {
        packet["queryCoverage"] = json!(query_coverage);
    }
    if !candidate_items.is_empty() {
        packet["candidateItems"] = json!(candidate_items);
    }
    packet
}

fn query_coverage_from_fields(query: &str, fields: &Map<String, Value>) -> (Value, Vec<Value>) {
    let value = string_field(fields, "itemQuery").unwrap_or_else(|| query.to_string());
    let status = string_field(fields, "status").unwrap_or_else(|| "miss".to_string());
    let match_kind = string_field(fields, "match").unwrap_or_else(|| "none".to_string());
    let match_count = usize_field(fields, "item").unwrap_or(0);
    let candidate_names = string_list_field(fields, "candidates");
    let next_action = string_field(fields, "next");
    let mut coverage = json!({
        "value": value,
        "status": status,
        "match": match_kind,
        "matchCount": match_count,
    });
    if !candidate_names.is_empty() {
        coverage["candidateNames"] = json!(candidate_names);
    }
    if let Some(next_action) = next_action.as_deref() {
        coverage["nextAction"] = json!(next_action);
    }
    let candidates = candidate_names
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "reason": "prefix",
                "term": value,
            })
        })
        .collect();
    (coverage, candidates)
}

fn item_match_from_line(line: &str, owner_path: Option<&str>) -> Option<Value> {
    let mut tokens = line.split_whitespace();
    let name = tokens.next()?;
    let fields = parse_fields(tokens);
    let read = string_field(&fields, "read");
    let (path, line, end_line) = read.as_deref().and_then(parse_read_locator).or_else(|| {
        let (line, end_line) = line_range_field(&fields)
            .or_else(|| usize_field(&fields, "line").map(|line| (line, line)))?;
        Some((owner_path?.to_string(), line, end_line))
    })?;
    let visibility = if bool_field(&fields, "public").unwrap_or(false) {
        "public"
    } else {
        "private"
    };
    let mut item = json!({
        "name": name,
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "item".to_string()),
        "visibility": visibility,
        "doc": bool_field(&fields, "doc").unwrap_or(false),
        "location": {
            "path": path,
            "lineRange": format!("{line}:{end_line}"),
        },
        "read": read.clone().unwrap_or_else(|| format!("{path}:{line}:{end_line}")),
        "patchSafety": {
            "level": "read-safe",
            "reason": "read exact source locator before editing this compact match",
            "exactRead": read.clone().unwrap_or_else(|| format!("{path}:{line}:{end_line}")),
        },
        "truncated": false,
    });
    if let Some(read) = read {
        item["read"] = json!(read);
    }
    Some(item)
}

fn attach_code_to_last_match(line: &str, matches: &mut [Value]) {
    let Some(item) = matches.last_mut() else {
        return;
    };
    let (field_text, text) = split_code_text(line);
    let fields = parse_fields(field_text.split_whitespace());
    if let Some(text) = text.as_deref() {
        item["code"] = json!(text);
    }
    if let Some(exact_read) = projection_exact_read(&fields) {
        let mut projection = json!({
            "mode": "compact",
            "syntax": "save-token-rustfmt",
            "sourceAuthority": "native-parser",
            "sourceFingerprint": projection_source_fingerprint(&exact_read, text.as_deref()),
            "compactSafety": {
                "literalPolicy": "summarize",
                "whitespacePolicy": "formatter-structural",
                "normalization": "none",
                "alignment": "parser-roundtrip",
                "exactReadRequired": true,
            },
            "losslessStructure": true,
            "exactRead": exact_read,
        });
        let source_fingerprint = string_field_value(&projection, "sourceFingerprint")
            .unwrap_or_else(|| projection_source_fingerprint(&exact_read, text.as_deref()));
        let mut nodes = projection_nodes_from_parser_fields(&fields, &exact_read, text.as_deref())
            .or_else(|| {
                text.as_deref()
                    .map(|text| projection_nodes_from_compact_code(&exact_read, text))
            })
            .unwrap_or_default();
        if !nodes.is_empty() {
            if item.get("code").is_none() {
                let compact_code = compact_code_from_projection_nodes(&nodes);
                if !compact_code.is_empty() {
                    item["code"] = json!(compact_code);
                }
            }
            let node_count = nodes.len();
            let nodes_truncated = node_count > MAX_QUERY_PACKET_PROJECTION_NODES;
            if nodes_truncated {
                nodes.truncate(MAX_QUERY_PACKET_PROJECTION_NODES);
                projection["losslessStructure"] = json!(false);
            }
            let compact_code = compact_code_from_projection_nodes(&nodes);
            if !compact_code.is_empty() {
                item["code"] = json!(compact_code);
            }
            let mut expand_actions = projection_expand_actions(&nodes);
            let expand_action_count = expand_actions.len();
            let expand_actions_truncated = expand_action_count > MAX_QUERY_PACKET_EXPAND_ACTIONS;
            if expand_actions_truncated {
                expand_actions.truncate(MAX_QUERY_PACKET_EXPAND_ACTIONS);
                projection["losslessStructure"] = json!(false);
            }
            projection["nodeCount"] = json!(node_count);
            projection["nodeLimit"] = json!(MAX_QUERY_PACKET_PROJECTION_NODES);
            projection["nodesTruncated"] = json!(nodes_truncated);
            if expand_actions_truncated {
                projection["expandActionCount"] = json!(expand_action_count);
                projection["expandActionLimit"] = json!(MAX_QUERY_PACKET_EXPAND_ACTIONS);
                projection["expandActionsTruncated"] = json!(true);
            }
            let rendered_node_ids = rendered_node_ids(&nodes);
            let rendered_rows = rendered_rows(&nodes, &rendered_node_ids);
            projection["renderedNodeIds"] = json!(rendered_node_ids);
            projection["renderedRows"] = json!(rendered_rows);
            if let Some(root_node_id) = nodes
                .first()
                .and_then(|node| string_field_value(node, "id"))
            {
                projection["omitted"] = json!([{
                    "kind": "source-formatting",
                    "reason": "compact projection removes original whitespace and comments",
                    "nodeId": root_node_id,
                    "read": exact_read,
                }]);
            }
            projection["nodes"] = json!(nodes);
            let semantic_responsibilities =
                projection_semantic_responsibilities(&fields, &nodes, &exact_read);
            if !semantic_responsibilities.is_empty() {
                projection["semanticResponsibilities"] = json!(semantic_responsibilities);
            }
            if !expand_actions.is_empty() {
                projection["expandActions"] = json!(expand_actions);
            }
            if !nodes_truncated
                && let Some(patch_safety) =
                    replace_item_patch_safety(item, &exact_read, &source_fingerprint)
            {
                item["patchSafety"] = patch_safety;
            }
        }
        item["projection"] = projection;
    }
    if let Some(truncated) = bool_field(&fields, "truncated") {
        item["truncated"] = json!(truncated);
    }
}

fn compact_code_from_projection_nodes(nodes: &[Value]) -> String {
    projection_code::compact_code_from_projection_nodes(nodes, |node| {
        Some((
            projection_node_depth(node),
            node["label"].as_str().unwrap_or_default().to_string(),
        ))
    })
}

fn projection_node_compact_text(node: &Value) -> Option<String> {
    let label = node["label"].as_str()?.trim();
    if label.is_empty() {
        return None;
    }
    let depth = projection_node_depth(node);
    Some(format!("{}{}", "    ".repeat(depth), label))
}

fn projection_node_depth(node: &Value) -> usize {
    node["depth"]
        .as_u64()
        .and_then(|depth| usize::try_from(depth).ok())
        .unwrap_or(0)
}

fn projection_nodes_from_compact_code(exact_read: &str, text: &str) -> Vec<Value> {
    text.lines()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .enumerate()
        .map(|(index, label)| {
            let classification = projection_node_classification(index, label);
            let mut node = json!({
                "id": format!("node:{index}"),
                "kind": classification.kind,
                "role": classification.role,
                "label": label,
                "depth": 0,
                "read": exact_read,
            });
            if !classification.flags.is_empty() {
                node["flags"] = json!(classification.flags);
            }
            node
        })
        .collect()
}

fn projection_nodes_from_parser_fields(
    fields: &Map<String, Value>,
    exact_read: &str,
    compact_text: Option<&str>,
) -> Option<Vec<Value>> {
    let (path, _, _) = parse_read_locator(exact_read)?;
    let mut parent_stack = Vec::<String>::new();
    let compact_labels = compact_text
        .map(compact_projection_labels)
        .unwrap_or_default();
    let nodes = string_list_field(fields, "nodes")
        .into_iter()
        .enumerate()
        .filter_map(|(index, token)| {
            projection_node_from_parser_token(
                &token,
                &path,
                compact_labels.get(index).map(String::as_str),
                &mut parent_stack,
            )
        })
        .collect::<Vec<_>>();
    (!nodes.is_empty()).then_some(nodes)
}

fn projection_node_from_parser_token(
    token: &str,
    path: &str,
    compact_label: Option<&str>,
    parent_stack: &mut Vec<String>,
) -> Option<Value> {
    let mut parts = token.split(':');
    let id = parts.next()?.to_string();
    let kind = parts.next()?.to_string();
    let role = parser_projection_role(parts.next()?)?.to_string();
    let depth = parts.next()?.parse::<usize>().ok()?;
    let line = parts.next()?.parse::<usize>().ok()?;
    let end_line = parts.next()?.parse::<usize>().ok()?;
    let native_id = parts.next()?.to_string();
    let structural_fingerprint = parts.next()?.to_string();
    if parts.next().is_some() || line == 0 || end_line < line {
        return None;
    }
    parent_stack.truncate(depth + 1);
    while parent_stack.len() <= depth {
        parent_stack.push(String::new());
    }
    let parent_id = depth
        .checked_sub(1)
        .and_then(|parent_depth| parent_stack.get(parent_depth))
        .filter(|parent_id| !parent_id.is_empty())
        .cloned();
    parent_stack[depth] = id.clone();

    let mut node = json!({
        "id": id,
        "nativeId": native_id,
        "structuralFingerprint": structural_fingerprint,
        "kind": kind,
        "role": role,
        "label": parser_projection_label(&kind, &role, compact_label),
        "depth": depth,
        "read": format!("{path}:{line}:{end_line}"),
    });
    if let Some(parent_id) = parent_id {
        node["parentId"] = json!(parent_id);
    }
    let flags = parser_projection_flags(&kind, &role);
    if !flags.is_empty() {
        node["flags"] = json!(flags);
    }
    Some(node)
}

fn rendered_node_ids(nodes: &[Value]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|node| string_field_value(node, "id"))
        .fold(Vec::new(), |mut ids, id| {
            if !ids.iter().any(|seen| seen == &id) {
                ids.push(id);
            }
            ids
        })
}

fn rendered_rows(nodes: &[Value], rendered_node_ids: &[String]) -> Vec<Value> {
    rendered_node_ids
        .iter()
        .filter_map(|node_id| {
            let node = nodes
                .iter()
                .find(|node| string_field_value(node, "id").as_deref() == Some(node_id.as_str()))?;
            let text = string_field_value(node, "label")?;
            let text = projection_node_compact_text(node).unwrap_or(text);
            let role = string_field_value(node, "role");
            Some(json!({
                "nodeId": node_id,
                "rowKind": rendered_row_kind(role.as_deref()),
                "text": text,
                "semanticWeight": rendered_row_weight(role.as_deref()),
            }))
        })
        .collect()
}

fn rendered_row_kind(role: Option<&str>) -> &'static str {
    match role {
        Some("declaration") => "declaration",
        Some("mutation") => "mutation",
        Some("call") => "call",
        Some("control-flow") => "control-flow",
        Some("terminal") => "terminal",
        Some("effect") => "effect",
        Some("field") => "field",
        _ => "unknown",
    }
}

fn rendered_row_weight(role: Option<&str>) -> usize {
    match role {
        Some("terminal" | "control-flow" | "mutation" | "call" | "effect") => 2,
        _ => 1,
    }
}

fn parser_projection_role(role: &str) -> Option<&'static str> {
    match role {
        "declaration" => Some("declaration"),
        "control-flow" => Some("control-flow"),
        "call" => Some("call"),
        "field" => Some("field"),
        "terminal" => Some("terminal"),
        "mutation" => Some("mutation"),
        "effect" => Some("effect"),
        _ => None,
    }
}

fn parser_projection_label(kind: &str, role: &str, compact_label: Option<&str>) -> String {
    if let Some(compact_label) = compact_label.filter(|label| !label.is_empty()) {
        compact_label.to_string()
    } else if role == "declaration" {
        format!("{kind} declaration")
    } else {
        kind.to_string()
    }
}

fn compact_projection_labels(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|label| {
            let label = label
                .strip_prefix('}')
                .map(str::trim_start)
                .unwrap_or(label);
            (!label.is_empty()).then(|| label.to_string())
        })
        .collect()
}

fn parser_projection_flags(kind: &str, role: &str) -> Vec<&'static str> {
    match role {
        "control-flow" if kind == "if" => vec!["branch", "guard"],
        "control-flow" if matches!(kind, "match" | "case" | "else") => vec!["branch"],
        "control-flow" if matches!(kind, "for" | "while" | "loop") => vec!["loop"],
        "call" => vec!["call"],
        "terminal" if kind == "return" => vec!["return"],
        "mutation" => vec!["mutation"],
        "effect" if kind == "await" => vec!["await", "effect"],
        "effect" => vec!["effect"],
        _ => Vec::new(),
    }
}

fn projection_expand_actions(nodes: &[Value]) -> Vec<Value> {
    nodes
        .iter()
        .filter_map(|node| {
            let role = node["role"].as_str()?;
            if matches!(role, "declaration" | "call") {
                return None;
            }
            let target = node["id"].as_str()?;
            let read = node["read"].as_str()?;
            Some(json!({
                "kind": "hot-block",
                "target": target,
                "read": read,
                "reason": format!("parser-projection-{role}"),
            }))
        })
        .collect()
}

fn projection_exact_read(fields: &Map<String, Value>) -> Option<String> {
    let path = string_field(fields, "path")?;
    let line_range = string_field(fields, "lineRange")?;
    Some(format!("{path}:{line_range}"))
}

fn projection_source_fingerprint(exact_read: &str, compact_text: Option<&str>) -> String {
    let text = compact_text.unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    exact_read.hash(&mut hasher);
    text.hash(&mut hasher);
    format!("{exact_read}:{}:{:016x}", text.len(), hasher.finish())
}

fn line_range_field(fields: &Map<String, Value>) -> Option<(usize, usize)> {
    let line_range = string_field(fields, "lineRange")?;
    let (start, end) = line_range.split_once(':')?;
    let start = start.parse::<usize>().ok()?;
    let end = end.parse::<usize>().ok()?;
    (start != 0 && end >= start).then_some((start, end))
}

fn split_code_text(line: &str) -> (&str, Option<String>) {
    let Some((fields, text_json)) = line.split_once(" text=") else {
        return (line, None);
    };
    (fields, serde_json::from_str::<String>(text_json).ok())
}

fn parse_read_locator(value: &str) -> Option<(String, usize, usize)> {
    let (path_and_start, end_line) = value.rsplit_once(':')?;
    let (path, line) = path_and_start.rsplit_once(':')?;
    Some((path.to_string(), line.parse().ok()?, end_line.parse().ok()?))
}

fn string_list_field(fields: &Map<String, Value>, key: &str) -> Vec<String> {
    fields.get(key).map_or_else(Vec::new, |value| match value {
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Value::String(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
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
