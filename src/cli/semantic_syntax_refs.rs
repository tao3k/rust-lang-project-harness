//! Tree-sitter-compatible syntax refs for semantic query/search packets.
#![allow(dead_code)]

use serde_json::{Value, json};

use crate::parser::syntax_abi::{
    RUST_OWNER_ITEMS_QUERY_REF, RustSyntaxAbi, rust_syntax_abi_for_kind,
};

pub(super) struct SemanticSyntaxRefs {
    pub(super) query_ref: String,
    pub(super) match_refs: Vec<String>,
    pub(super) capture_refs: Vec<String>,
    pub(super) anchor: Option<Value>,
}

struct MatchSyntaxRef {
    match_ref: String,
    capture_ref: String,
    anchor: Value,
}

impl SemanticSyntaxRefs {
    fn new() -> Self {
        Self {
            query_ref: RUST_OWNER_ITEMS_QUERY_REF.to_string(),
            match_refs: Vec::new(),
            capture_refs: Vec::new(),
            anchor: None,
        }
    }

    fn push(mut self, syntax_ref: MatchSyntaxRef) -> Self {
        self.match_refs.push(syntax_ref.match_ref);
        self.capture_refs.push(syntax_ref.capture_ref);
        self.anchor.get_or_insert(syntax_ref.anchor);
        self
    }

    fn into_option(self) -> Option<Self> {
        (!self.match_refs.is_empty()).then_some(self)
    }
}

pub(super) fn attach_syntax_refs_to_matches(matches: &mut [Value]) -> Option<SemanticSyntaxRefs> {
    matches
        .iter_mut()
        .enumerate()
        .filter_map(|(index, item)| attach_syntax_ref_to_match(index, item))
        .fold(SemanticSyntaxRefs::new(), SemanticSyntaxRefs::push)
        .into_option()
}

pub(super) fn attach_syntax_refs_to_search_items(
    items: &mut [Value],
) -> Option<SemanticSyntaxRefs> {
    items
        .iter_mut()
        .enumerate()
        .filter_map(|(index, item)| attach_syntax_ref_to_search_item(index, item))
        .fold(SemanticSyntaxRefs::new(), SemanticSyntaxRefs::push)
        .into_option()
}

pub(super) fn attach_syntax_refs_to_source_windows(
    source_windows: &mut [Value],
) -> Option<SemanticSyntaxRefs> {
    source_windows
        .iter_mut()
        .enumerate()
        .filter_map(|(index, window)| attach_syntax_ref_to_source_window(index, window))
        .fold(SemanticSyntaxRefs::new(), SemanticSyntaxRefs::push)
        .into_option()
}

pub(super) fn syntax_refs_from_read_plan_symbols(read_plan: &Value) -> Option<SemanticSyntaxRefs> {
    read_plan
        .get("symbols")?
        .as_array()?
        .iter()
        .enumerate()
        .filter_map(|(index, symbol)| syntax_ref_from_read_plan_symbol(index, symbol))
        .fold(SemanticSyntaxRefs::new(), SemanticSyntaxRefs::push)
        .into_option()
}

fn attach_syntax_ref_to_match(index: usize, item: &mut Value) -> Option<MatchSyntaxRef> {
    let location = item.get("location").cloned().filter(Value::is_object)?;
    attach_syntax_ref_to_item(index, item, location)
}

fn attach_syntax_ref_to_search_item(index: usize, item: &mut Value) -> Option<MatchSyntaxRef> {
    let location = search_item_syntax_location(item)?;
    attach_syntax_ref_to_item(index, item, location)
}

fn attach_syntax_ref_to_source_window(index: usize, window: &mut Value) -> Option<MatchSyntaxRef> {
    let kind = window.get("itemKind").and_then(Value::as_str)?.to_string();
    let location = window.get("location").cloned().filter(Value::is_object)?;
    attach_syntax_ref(index, window, &kind, location)
}

fn attach_syntax_ref_to_item(
    index: usize,
    item: &mut Value,
    location: Value,
) -> Option<MatchSyntaxRef> {
    let kind = item
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("item")
        .to_string();
    attach_syntax_ref(index, item, &kind, location)
}

fn attach_syntax_ref(
    index: usize,
    item: &mut Value,
    kind: &str,
    location: Value,
) -> Option<MatchSyntaxRef> {
    let syntax = rust_syntax_abi_for_kind(kind);
    let match_ref = format!("match.{}", index + 1);
    let capture_ref = format!("capture.{}", index + 1);

    ensure_fields_object(item);
    item["fields"]["syntaxQueryRef"] = json!(RUST_OWNER_ITEMS_QUERY_REF);
    item["fields"]["syntaxMatchRef"] = json!(match_ref);
    item["fields"]["syntaxCaptureRef"] = json!(capture_ref);
    item["fields"]["syntaxNodeType"] = json!(syntax.node_type);
    item["fields"]["syntaxCapture"] = json!(syntax.capture);

    Some(MatchSyntaxRef {
        match_ref,
        capture_ref,
        anchor: syntax_anchor(syntax, location),
    })
}

fn syntax_ref_from_read_plan_symbol(index: usize, symbol: &Value) -> Option<MatchSyntaxRef> {
    let kind = symbol.get("itemKind").and_then(Value::as_str)?;
    let read = symbol.get("read").and_then(Value::as_str)?;
    let syntax = rust_syntax_abi_for_kind(kind);
    let match_ref = format!("match.{}", index + 1);
    let capture_ref = format!("capture.{}", index + 1);
    Some(MatchSyntaxRef {
        match_ref,
        capture_ref,
        anchor: syntax_anchor(syntax, location_from_read_locator(read)?),
    })
}

fn search_item_syntax_location(item: &Value) -> Option<Value> {
    let fields = item.get("fields").and_then(Value::as_object)?;
    let read = fields.get("read").and_then(Value::as_str)?;
    location_from_read_locator(read)
}

fn location_from_read_locator(read: &str) -> Option<Value> {
    let (path_and_start, end) = read.rsplit_once(':')?;
    let (path, start) = path_and_start.rsplit_once(':')?;
    if path.is_empty() || start.is_empty() || end.is_empty() {
        return None;
    }
    Some(json!({
        "path": path,
        "lineRange": format!("{start}:{end}"),
    }))
}

fn ensure_fields_object(item: &mut Value) {
    if !item.get("fields").is_some_and(Value::is_object) {
        item["fields"] = json!({});
    }
}

fn syntax_anchor(syntax: RustSyntaxAbi, location: Value) -> Value {
    let mut anchor = json!({
        "nodeType": syntax.node_type,
        "capture": syntax.capture,
        "location": location,
    });
    if let Some(field) = syntax.field {
        anchor["field"] = json!(field);
    }
    anchor
}
