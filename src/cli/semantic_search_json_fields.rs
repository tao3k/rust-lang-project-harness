use std::path::Path;

use serde_json::{Map, Value, json};

pub(super) fn parse_fields<'a>(tokens: impl IntoIterator<Item = &'a str>) -> Map<String, Value> {
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

pub(super) fn next_field(fields: &Map<String, Value>) -> Option<String> {
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

pub(super) fn parse_next_actions(fragment: String, owner_path: Option<&str>) -> Vec<Value> {
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

pub(super) fn parse_edge_kind(token: &str) -> (String, Option<String>) {
    let edge = token.trim_start_matches('-').trim_end_matches("->");
    edge.split_once(':')
        .map(|(kind, label)| (kind.to_string(), Some(label.to_string())))
        .unwrap_or_else(|| (edge.to_string(), None))
}

pub(super) fn input_detection_from_header(fields: &Map<String, Value>) -> Option<Value> {
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

pub(super) fn header_package(packet: &Value) -> Option<&str> {
    packet["header"]["fields"]["package"].as_str()
}

pub(super) fn string_field(fields: &Map<String, Value>, key: &str) -> Option<String> {
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

pub(super) fn bool_field(fields: &Map<String, Value>, key: &str) -> Option<bool> {
    fields.get(key).and_then(Value::as_bool)
}

fn usize_field(fields: &Map<String, Value>, key: &str) -> Option<usize> {
    fields
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

pub(super) fn insert_if_some(fields: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.insert(key.to_string(), Value::String(value.to_string()));
    }
}

pub(super) fn insert_if_usize(fields: &mut Map<String, Value>, key: &str, value: Option<usize>) {
    if let Some(value) = value {
        fields.insert(key.to_string(), json!(value));
    }
}

pub(super) fn location_from_node(node: &str) -> Value {
    location(node.strip_prefix("O:").unwrap_or(node))
}

pub(super) fn location(path: &str) -> Value {
    json!({ "path": path })
}

pub(super) fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
