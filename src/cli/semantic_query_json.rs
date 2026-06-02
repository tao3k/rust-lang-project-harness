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
        Some((
            owner_path?.to_string(),
            usize_field(&fields, "line")?,
            usize_field(&fields, "endLine").or_else(|| usize_field(&fields, "line"))?,
        ))
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
            "line": line,
            "endLine": end_line,
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
        let nodes = text
            .as_deref()
            .map(|text| projection_nodes_from_compact_code(&exact_read, text))
            .unwrap_or_default();
        if !nodes.is_empty() {
            projection["nodes"] = json!(nodes);
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
    let start_line = usize_field(fields, "startLine")?;
    let end_line = usize_field(fields, "endLine")?;
    Some(format!("{path}:{start_line}-{end_line}"))
}

fn split_code_text(line: &str) -> (&str, Option<String>) {
    let Some((fields, text_json)) = line.split_once(" text=") else {
        return (line, None);
    };
    (fields, serde_json::from_str::<String>(text_json).ok())
}

fn parse_read_locator(value: &str) -> Option<(String, usize, usize)> {
    let (path, range) = value.rsplit_once(':')?;
    let (line, end_line) = range
        .split_once('-')
        .map_or((range, range), |(line, end_line)| (line, end_line));
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
