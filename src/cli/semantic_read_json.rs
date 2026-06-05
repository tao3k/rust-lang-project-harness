use std::path::Path;

use serde_json::{Map, Value, json};

use super::semantic_search_json_fields::{display_path, parse_fields, string_field};
use super::semantic_syntax_refs::{
    attach_syntax_refs_to_source_windows, syntax_refs_from_read_plan_symbols,
};

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
    let read_plan = read_plan_from_rendered(rendered);
    let selector_range = parse_read_locator(&options.selector).map(|(_, start, end)| (start, end));
    let mut last_window_accepts_code = false;
    for line in rendered.lines().filter(|line| line.starts_with('|')) {
        if let Some(rest) = line.strip_prefix("|owner ") {
            owner_path = rest.split_whitespace().next().map(ToOwned::to_owned);
        } else if let Some(rest) = line.strip_prefix("|item ") {
            if let Some(window) = source_window_from_item_line(rest, owner_path.as_deref()) {
                last_window_accepts_code = window_overlaps_selector(&window, selector_range);
                if last_window_accepts_code {
                    source_windows.push(window)
                }
            }
        } else if last_window_accepts_code && let Some(rest) = line.strip_prefix("|code ") {
            attach_text_to_last_window(rest, &mut source_windows)
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
        "truncated": false,
        "notes": [],
    });
    let syntax_refs = if let Some(read_plan) = read_plan {
        let syntax_refs = syntax_refs_from_read_plan_symbols(&read_plan);
        packet["readPlan"] = read_plan;
        syntax_refs
    } else {
        if source_windows.is_empty()
            && let Some((path, start_line, end_line)) = parse_read_locator(&options.selector)
        {
            let text = rendered.trim_end_matches('\n').to_string();
            if !text.trim().is_empty() {
                let line_range = format!("{start_line}:{end_line}");
                let lines = text
                    .lines()
                    .enumerate()
                    .map(|(offset, text)| json!({ "number": start_line + offset, "text": text }))
                    .collect::<Vec<_>>();
                let line_count = lines.len();
                source_windows.push(json!({
                    "ownerPath": path.clone(),
                    "location": { "path": path.clone(), "lineRange": line_range },
                    "read": format!("{path}:{line_range}"),
                    "lineCount": line_count,
                    "reason": "direct-selector",
                    "text": text,
                    "lines": lines,
                    "truncated": false,
                }));
            }
        }
        let syntax_refs = attach_syntax_refs_to_source_windows(&mut source_windows);
        packet["sourceWindows"] = json!(source_windows);
        syntax_refs
    };
    if let Some(owner_path) = owner_path {
        packet["ownerPath"] = json!(owner_path);
    }
    if let Some(query) = &options.query {
        packet["query"] = json!(query);
        packet["queryTerms"] = json!(query_terms(query));
    }
    if let Some(syntax_refs) = syntax_refs {
        packet["syntaxQueryRef"] = json!(syntax_refs.query_ref);
        packet["syntaxMatchRefs"] = json!(syntax_refs.match_refs);
        packet["syntaxCaptureRefs"] = json!(syntax_refs.capture_refs);
        if let Some(anchor) = syntax_refs.anchor {
            packet["syntaxAnchor"] = anchor;
        }
    }
    packet
}

fn read_plan_from_rendered(rendered: &str) -> Option<Value> {
    let header = rendered
        .lines()
        .find_map(|line| line.strip_prefix("[read-plan] "))?;
    let header_fields = parse_fields(header.split_whitespace());
    let rows = ReadPlanRows::collect(rendered);
    if rows.windows.is_empty() && rows.symbols.is_empty() {
        return None;
    }
    let frontier = read_plan_frontier(&rows)?;
    let mut read_plan = json!({
            "mode": string_field(&header_fields, "mode").unwrap_or_else(|| "range-frontier".to_string()),
    "code": false,
    "reason": string_field(&header_fields, "reason").unwrap_or_else(|| "wide-selector".to_string()),
    "frontier": frontier,
    "avoid": ["repeat-wide-read", "manual-window-scan", "raw-read"],
    "omit": ["code"],
    });
    if !rows.windows.is_empty() {
        read_plan["windows"] = json!(rows.windows);
    }
    if !rows.symbols.is_empty() {
        read_plan["symbols"] = json!(rows.symbols);
    }
    if !rows.ranges.is_empty() {
        read_plan["ranges"] = json!(rows.ranges);
    }
    if let Some(max_window_lines) = usize_field(&header_fields, "maxWindow") {
        read_plan["maxWindowLines"] = json!(max_window_lines);
    }
    if let Some(algorithm) = string_field(&header_fields, "alg") {
        read_plan["algorithm"] = json!(algorithm);
    }
    if let Some(syn) = string_field(&header_fields, "syn") {
        read_plan["syn"] = json!(syn);
    }
    Some(read_plan)
}

#[derive(Default)]
struct ReadPlanRows {
    ranges: Vec<Value>,
    symbols: Vec<Value>,
    windows: Vec<Value>,
}

impl ReadPlanRows {
    fn collect(rendered: &str) -> Self {
        rendered.lines().fold(Self::default(), |mut rows, line| {
            rows.record_graph_line(line);
            rows.record_legacy_line(line);
            rows
        })
    }

    fn record_graph_line(&mut self, line: &str) {
        line.split(';').for_each(|fragment| {
            let _ = push_read_plan_graph_fragment(
                fragment,
                &mut self.ranges,
                &mut self.symbols,
                &mut self.windows,
            );
        });
    }

    fn record_legacy_line(&mut self, line: &str) -> Option<()> {
        if let Some(rest) = line.strip_prefix("|range ") {
            let fields = parse_fields(rest.split_whitespace());
            self.ranges.push(json!({
                "path": string_field(&fields, "path")?,
                "requested": string_field(&fields, "requested")?,
                "selected": string_field(&fields, "selected")?,
                "matched": string_field(&fields, "matched")?,
                "coverage": string_field(&fields, "coverage").unwrap_or_else(|| "full".to_string()),
                "density": string_field(&fields, "density").unwrap_or_else(|| "unknown".to_string()),
            }));
        } else if let Some(rest) = line.strip_prefix("|window ") {
            let fields = parse_fields(rest.split_whitespace());
            self.windows.push(json!({
                "path": string_field(&fields, "path")?,
                "lineRange": string_field(&fields, "lineRange")?,
                "read": string_field(&fields, "read")?,
                "lineCount": usize_field(&fields, "lineCount")?,
                "reason": string_field(&fields, "reason").unwrap_or_else(|| "split".to_string()),
            }));
        } else if let Some(rest) = line.strip_prefix("|symbol ") {
            let fields = parse_fields(rest.split_whitespace());
            self.symbols.push(json!({
                "itemName": string_field(&fields, "item")?,
                "itemKind": string_field(&fields, "kind")?,
                "lineRange": string_field(&fields, "lineRange")?,
                "read": string_field(&fields, "read")?,
            }));
        }
        Some(())
    }
}

fn read_plan_frontier(rows: &ReadPlanRows) -> Option<Vec<Value>> {
    let frontier_source = if rows.symbols.is_empty() {
        ("window", &rows.windows)
    } else {
        ("symbol", &rows.symbols)
    };
    frontier_source
        .1
        .iter()
        .enumerate()
        .map(|(index, target)| read_plan_frontier_entry(frontier_source.0, index, target))
        .collect()
}

fn read_plan_frontier_entry(kind: &str, index: usize, target: &Value) -> Option<Value> {
    let read = target.get("read")?.as_str()?;
    let (path, start_line, end_line) = parse_read_locator(read)?;
    let line_range = format!("{start_line}:{end_line}");
    Some(json!({
        "id": read_plan_frontier_id(kind, index),
        "kind": kind,
        "target": format!("{path}@{line_range}"),
        "read": read,
        "action": "code",
        "rank": index + 1,
        "reason": if kind == "symbol" { "parser-item" } else { "split" },
    }))
}

fn read_plan_frontier_id(kind: &str, index: usize) -> String {
    match (kind, index) {
        ("symbol", 0) => "S".to_string(),
        ("window", 0) => "W".to_string(),
        ("symbol", _) => format!("S{}", index + 1),
        _ => format!("W{}", index + 1),
    }
}

fn push_read_plan_graph_fragment(
    fragment: &str,
    ranges: &mut Vec<Value>,
    symbols: &mut Vec<Value>,
    windows: &mut Vec<Value>,
) -> Option<()> {
    let fragment = fragment.trim();
    let (_id, payload) = fragment.split_once('=')?;
    let (typed_target, action) = payload.rsplit_once('!')?;
    let (kind, role_target) = typed_target.split_once(':')?;
    let (role, target, locator) = graph_alias_role_target(role_target)?;
    match (kind, role, action) {
        ("range", "requested", "outline") => {
            let (path, line_range, _read, _line_count) = graph_target_read(target)?;
            ranges.push(json!({
                "path": path,
                "requested": line_range,
                "selected": line_range,
                "matched": line_range,
                "coverage": "full",
                "density": "unknown",
            }));
        }
        ("window", "range", "code") => {
            let (path, line_range, read, line_count) = graph_target_read(target)?;
            windows.push(json!({
                "path": path,
                "lineRange": line_range,
                "read": read,
                "lineCount": line_count,
                "reason": "split",
            }));
        }
        ("symbol", item_kind, "code") => {
            let read = locator?;
            let (_path, start_line, end_line) = parse_read_locator(read)?;
            symbols.push(json!({
                "itemName": target,
                "itemKind": item_kind,
                "lineRange": format!("{start_line}:{end_line}"),
                "read": read,
            }));
        }
        _ => {}
    }
    Some(())
}

fn graph_alias_role_target(value: &str) -> Option<(&str, &str, Option<&str>)> {
    let open = value.find('(')?;
    let role = &value[..open];
    let rest = &value[open + 1..];
    let close = rest.find(')')?;
    let target = &rest[..close];
    let locator = rest[close + 1..].strip_prefix('@');
    Some((role, target, locator))
}

fn graph_target_read(target: &str) -> Option<(String, String, String, usize)> {
    let (path, line_range) = target.rsplit_once('@')?;
    let read = format!("{path}:{line_range}");
    let (_path, start_line, end_line) = parse_read_locator(&read)?;
    Some((
        path.to_string(),
        line_range.to_string(),
        read,
        end_line.saturating_sub(start_line).saturating_add(1),
    ))
}

fn source_window_from_item_line(line: &str, owner_path: Option<&str>) -> Option<Value> {
    let mut tokens = line.split_whitespace();
    let name = tokens.next()?;
    let fields = parse_fields(tokens);
    let read = string_field(&fields, "read");
    let (path, start_line, end_line) =
        read.as_deref().and_then(parse_read_locator).or_else(|| {
            let (start_line, end_line) = line_range_field(&fields)
                .or_else(|| usize_field(&fields, "line").map(|line| (line, line)))?;
            Some((owner_path?.to_string(), start_line, end_line))
        })?;
    let line_count = end_line.saturating_sub(start_line).saturating_add(1);
    let mut window = json!({
        "ownerPath": path,
        "itemName": name,
        "itemKind": string_field(&fields, "kind").unwrap_or_else(|| "item".to_string()),
            "location": {
                "path": path,
                "lineRange": format!("{start_line}:{end_line}"),
            },
            "lineCount": line_count,
        "reason": "direct-selector",
        "truncated": false,
    });
    if let Some(read) = read {
        window["read"] = json!(read);
    }
    Some(window)
}

fn window_overlaps_selector(window: &Value, selector_range: Option<(usize, usize)>) -> bool {
    let Some((selector_start, selector_end)) = selector_range else {
        return true;
    };
    let Some((window_start, window_end)) = window_line_range(window) else {
        return false;
    };
    window_end >= selector_start && window_start <= selector_end
}

fn window_line_range(window: &Value) -> Option<(usize, usize)> {
    let line_range = window.get("location")?.get("lineRange")?.as_str()?;
    let (start, end) = line_range.split_once(':')?;
    Some((start.parse().ok()?, end.parse().ok()?))
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
    let value = ["owner:", "read:", "path:"]
        .into_iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .unwrap_or(value);
    let (path_and_start, end_part) = value.rsplit_once(':')?;
    if let Some((start, end)) = end_part.split_once('-') {
        let start_line = start.parse().ok()?;
        let end_line = end.parse().ok()?;
        return (start_line != 0 && end_line >= start_line).then_some((
            path_and_start.to_string(),
            start_line,
            end_line,
        ));
    }
    let (path, start) = path_and_start.rsplit_once(':')?;
    let start_line = start.parse().ok()?;
    let end_line = end_part.parse().ok()?;
    (start_line != 0 && end_line >= start_line).then_some((path.to_string(), start_line, end_line))
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
