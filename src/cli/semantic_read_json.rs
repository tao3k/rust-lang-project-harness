use std::path::Path;

use serde_json::{Map, Value, json};

use super::semantic_search_json_fields::{display_path, parse_fields, string_field};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-read-packet";
const SCHEMA_VERSION: &str = "1";

pub(super) struct SemanticReadJsonOptions {
    pub(super) selector: String,
    pub(super) query: Option<String>,
}

pub(super) fn render_read_json(
    project_root: &Path,
    options: &SemanticReadJsonOptions,
    rendered: &str,
) -> Result<String, String> {
    let packet = build_packet(project_root, options, rendered);
    serde_json::to_string(&packet).map_err(|error| format!("failed to render read JSON: {error}"))
}

fn build_packet(project_root: &Path, options: &SemanticReadJsonOptions, rendered: &str) -> Value {
    let mut owner_path = None::<String>;
    let mut source_windows = Vec::<Value>::new();

    for line in rendered.lines().filter(|line| line.starts_with('|')) {
        if let Some(rest) = line.strip_prefix("|owner ") {
            owner_path = rest.split_whitespace().next().map(ToOwned::to_owned);
        } else if let Some(rest) = line.strip_prefix("|item ") {
            if let Some(window) = source_window_from_item_line(rest, owner_path.as_deref()) {
                source_windows.push(window);
            }
        } else if let Some(rest) = line.strip_prefix("|code ") {
            attach_text_to_last_window(rest, &mut source_windows);
        }
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
        "method": "query/direct-source-read",
        "projectRoot": display_path(project_root),
        "selector": options.selector,
        "fromHook": "direct-source-read",
        "outputMode": "read-packet",
        "sourceWindows": source_windows,
        "truncated": false,
        "notes": [],
    });

    if let Some(owner_path) = owner_path {
        packet["ownerPath"] = json!(owner_path);
    }
    if let Some(query) = &options.query {
        packet["query"] = json!(query);
        packet["queryTerms"] = json!(query_terms(query));
    }

    packet
}

fn source_window_from_item_line(line: &str, owner_path: Option<&str>) -> Option<Value> {
    let mut tokens = line.split_whitespace();
    let name = tokens.next()?;
    let fields = parse_fields(tokens);
    let read = string_field(&fields, "read");
    let (path, start_line, end_line) =
        read.as_deref().and_then(parse_read_locator).or_else(|| {
            Some((
                owner_path?.to_string(),
                usize_field(&fields, "line")?,
                usize_field(&fields, "endLine").or_else(|| usize_field(&fields, "line"))?,
            ))
        })?;
    let line_count = end_line.saturating_sub(start_line).saturating_add(1);
    let mut window = json!({
        "ownerPath": path,
        "itemName": name,
        "itemKind": string_field(&fields, "kind").unwrap_or_else(|| "item".to_string()),
        "location": {
            "path": path,
            "line": start_line,
            "endLine": end_line,
        },
        "startLine": start_line,
        "endLine": end_line,
        "lineCount": line_count,
        "reason": "direct-selector",
        "truncated": false,
    });
    if let Some(read) = read {
        window["read"] = json!(read);
    }
    Some(window)
}

fn attach_text_to_last_window(line: &str, source_windows: &mut [Value]) {
    let Some(window) = source_windows.last_mut() else {
        return;
    };
    let (field_text, text) = split_code_text(line);
    let fields = parse_fields(field_text.split_whitespace());
    if let Some(text) = text {
        window["text"] = json!(text);
    }
    if let Some(read) = projection_exact_read(&fields) {
        window["read"] = json!(read);
    }
    if let Some(truncated) = bool_field(&fields, "truncated") {
        window["truncated"] = json!(truncated);
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

fn query_terms(query: &str) -> Vec<&str> {
    query
        .split('|')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect()
}

fn bool_field(fields: &Map<String, Value>, key: &str) -> Option<bool> {
    fields.get(key)?.as_bool()
}

fn usize_field(fields: &Map<String, Value>, key: &str) -> Option<usize> {
    fields.get(key)?.as_u64()?.try_into().ok()
}
