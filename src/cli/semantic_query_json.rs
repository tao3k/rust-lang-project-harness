//! JSON renderer for provider-native parser query packets.

use std::path::Path;

use serde_json::{Map, Value, json};

use super::semantic_search_json_fields::{display_path, parse_fields, string_field};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-query-packet";
const SCHEMA_VERSION: &str = "1";

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
        "matches": matches,
        "truncated": false,
    });
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
            "syntax": "semantic-outline",
            "sourceAuthority": "native-parser",
            "losslessStructure": true,
            "exactRead": exact_read,
        });
        let nodes = projection_nodes_from_parser_fields(&fields, &exact_read, text.as_deref())
            .or_else(|| {
                text.as_deref()
                    .map(|text| projection_nodes_from_compact_code(&exact_read, text))
            })
            .unwrap_or_default();
        if !nodes.is_empty() {
            let expand_actions = projection_expand_actions(&nodes);
            projection["nodes"] = json!(nodes);
            if !expand_actions.is_empty() {
                projection["expandActions"] = json!(expand_actions);
            }
        }
        item["projection"] = projection;
    }
    if let Some(truncated) = bool_field(&fields, "truncated") {
        item["truncated"] = json!(truncated);
    }
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

fn parser_projection_role(role: &str) -> Option<&'static str> {
    match role {
        "declaration" => Some("declaration"),
        "control-flow" => Some("control-flow"),
        "call" => Some("call"),
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
        .filter(|label| !label.is_empty())
        .map(ToOwned::to_owned)
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

struct ProjectionNodeClassification {
    kind: &'static str,
    role: &'static str,
    flags: &'static [&'static str],
}

fn projection_node_classification(index: usize, label: &str) -> ProjectionNodeClassification {
    if index == 0 {
        return ProjectionNodeClassification {
            kind: declaration_projection_kind(label),
            role: "declaration",
            flags: &[],
        };
    }
    if label.starts_with("if ") {
        return ProjectionNodeClassification {
            kind: "if",
            role: "control-flow",
            flags: &["branch", "guard"],
        };
    }
    if label == "else" || label.starts_with("case ") || label.starts_with("match ") {
        return ProjectionNodeClassification {
            kind: if label == "else" {
                "else"
            } else if label.starts_with("case ") {
                "case"
            } else {
                "match"
            },
            role: "control-flow",
            flags: &["branch"],
        };
    }
    if label == "loop" || label.starts_with("for ") || label.starts_with("while ") {
        return ProjectionNodeClassification {
            kind: if label == "loop" {
                "loop"
            } else if label.starts_with("for ") {
                "for"
            } else {
                "while"
            },
            role: "control-flow",
            flags: &["loop"],
        };
    }
    if label.starts_with("call ") {
        return ProjectionNodeClassification {
            kind: "call",
            role: "call",
            flags: &["call"],
        };
    }
    if label == "break" || label.starts_with("break ") {
        return ProjectionNodeClassification {
            kind: "break",
            role: "terminal",
            flags: &[],
        };
    }
    if label == "continue" {
        return ProjectionNodeClassification {
            kind: "continue",
            role: "terminal",
            flags: &[],
        };
    }
    if label.starts_with("return ") || label == "return" {
        return ProjectionNodeClassification {
            kind: "return",
            role: "terminal",
            flags: &["return"],
        };
    }
    if label.starts_with("tail ") {
        return ProjectionNodeClassification {
            kind: "tail",
            role: "terminal",
            flags: &[],
        };
    }
    if label.starts_with("assign ") || label.starts_with("let ") {
        return ProjectionNodeClassification {
            kind: if label.starts_with("assign ") {
                "assign"
            } else {
                "let"
            },
            role: "mutation",
            flags: &["mutation"],
        };
    }
    if label.starts_with("await ") || label.starts_with("try ") {
        return ProjectionNodeClassification {
            kind: if label.starts_with("await ") {
                "await"
            } else {
                "try"
            },
            role: "effect",
            flags: &["effect"],
        };
    }
    ProjectionNodeClassification {
        kind: "unknown",
        role: "unknown",
        flags: &[],
    }
}

fn declaration_projection_kind(label: &str) -> &'static str {
    if label.contains(" fn ") || label.starts_with("fn ") || label.starts_with("pub fn ") {
        "fn"
    } else if label.contains(" struct ")
        || label.starts_with("struct ")
        || label.starts_with("pub struct ")
    {
        "struct"
    } else if label.contains(" enum ")
        || label.starts_with("enum ")
        || label.starts_with("pub enum ")
    {
        "enum"
    } else if label.contains(" trait ")
        || label.starts_with("trait ")
        || label.starts_with("pub trait ")
    {
        "trait"
    } else {
        "item"
    }
}

fn projection_exact_read(fields: &Map<String, Value>) -> Option<String> {
    let path = string_field(fields, "path")?;
    let line_range = string_field(fields, "lineRange")?;
    Some(format!("{path}:{line_range}"))
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
