pub(super) struct SyntaxQueryRow {
    pub(super) capture: String,
    pub(super) capture_node: String,
    pub(super) capture_field: String,
    pub(super) capture_text: String,
    pub(super) node: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
    pub(super) item_start_line: usize,
    pub(super) item_end_line: usize,
    pub(super) item_code: String,
}

pub(super) fn syntax_query_matches_json(rows: &[SyntaxQueryRow]) -> Vec<serde_json::Value> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let ordinal = index + 1;
            let native_fact_ref = syntax_query_native_fact_ref(row);
            let semantic_handle_ref = syntax_query_semantic_handle_ref(row);
            let match_source_location =
                syntax_query_source_location(row, row.item_start_line, row.item_end_line);
            let capture_source_location =
                syntax_query_source_location(row, row.start_line, row.end_line);
            serde_json::json!({
                "id": format!("match.{ordinal}"),
                "patternIndex": 0,
                "range": {
                    "path": row.path.as_str(),
                    "lineRange": syntax_query_line_range(row.item_start_line, row.item_end_line)
                },
                "sourceLocation": match_source_location,
                "captures": [{
                    "id": format!("capture.{ordinal}"),
                    "name": row.capture.as_str(),
                    "nodeType": row.capture_node,
                    "field": row.capture_field.as_str(),
                    "named": true,
                    "range": {
                        "path": row.path.as_str(),
                        "lineRange": syntax_query_line_range(row.start_line, row.end_line)
                    },
                    "sourceLocation": capture_source_location,
                    "nativeFactRefs": [native_fact_ref.clone()],
                    "semanticHandleRefs": [semantic_handle_ref.clone()],
                    "fields": {
                        "symbol": row.name.as_str(),
                        "read": syntax_query_read_locator(row),
                        "itemRead": syntax_query_item_read_locator(row),
                        "sourceAuthority": "native-parser",
                        "nativeNodeType": row.node,
                        "semanticKind": syntax_query_semantic_kind(&row.node)
                    }
                }],
                "nativeFactRefs": [native_fact_ref],
                "semanticHandleRefs": [semantic_handle_ref],
                "fields": {
                    "symbol": row.name.as_str(),
                    "read": syntax_query_read_locator(row),
                    "itemRead": syntax_query_item_read_locator(row),
                    "nodeType": row.node,
                    "captureCount": 1
                }
            })
        })
        .collect()
}

pub(super) fn syntax_query_native_fact_refs(rows: &[SyntaxQueryRow]) -> Vec<String> {
    rows.iter()
        .map(syntax_query_native_fact_ref)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn syntax_query_native_fact_ref(row: &SyntaxQueryRow) -> String {
    let fact_kind = if row.node == "call_expression" {
        "syntax"
    } else {
        "item"
    };
    format!(
        "rust:{fact_kind}:{}:{}:{}",
        row.path,
        syntax_query_line_range(row.item_start_line, row.item_end_line),
        row.name
    )
}

fn syntax_query_semantic_handle_ref(row: &SyntaxQueryRow) -> String {
    format!("symbol:{}", row.name)
}

fn syntax_query_read_locator(row: &SyntaxQueryRow) -> String {
    format!(
        "{}:{}",
        row.path,
        syntax_query_line_range(row.start_line, row.end_line)
    )
}

fn syntax_query_item_read_locator(row: &SyntaxQueryRow) -> String {
    format!(
        "{}:{}",
        row.path,
        syntax_query_line_range(row.item_start_line, row.item_end_line)
    )
}

fn syntax_query_source_location(
    row: &SyntaxQueryRow,
    start_line: usize,
    end_line: usize,
) -> serde_json::Value {
    let line_range = syntax_query_line_range(start_line, end_line);
    let source_span_locator = format!("{}:{}", row.path, line_range);
    serde_json::json!({
        "path": row.path.as_str(),
        "lineRange": line_range.as_str(),
        "location": {
            "path": row.path.as_str(),
            "lineRange": line_range.as_str(),
        },
        "sourceLocator": source_span_locator.as_str(),
        "sourceSpanLocator": source_span_locator.as_str()
    })
}

fn syntax_query_line_range(start_line: usize, end_line: usize) -> String {
    format!("{}:{}", start_line.max(1), end_line.max(start_line).max(1))
}

fn syntax_query_semantic_kind(node: &str) -> &'static str {
    match node {
        "const_item" | "static_item" => "constant",
        "call_expression" => "call",
        "enum_item" => "enum",
        "extern_crate_declaration" => "extern",
        "function_item" => "function",
        "impl_item" => "impl",
        "macro_definition" | "macro_invocation" => "macro",
        "mod_item" => "module",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "type_item" => "type",
        "use_declaration" => "import",
        _ => "item",
    }
}
