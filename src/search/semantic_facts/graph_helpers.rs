//! Shared semantic graph JSON helpers.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};

const HOT_CONTEXT_BEFORE_LINES: usize = 8;
const HOT_CONTEXT_AFTER_LINES: usize = 12;

pub(super) fn push_node(nodes: &mut Vec<Value>, seen_nodes: &mut BTreeSet<String>, node: Value) {
    let Some(id) = node.get("id").and_then(Value::as_str) else {
        return;
    };
    if seen_nodes.insert(id.to_string()) {
        nodes.push(node);
    }
}

pub(super) fn push_edge(
    edges: &mut Vec<Value>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
    source: &str,
    target: &str,
    relation: &str,
) {
    let key = (source.to_string(), target.to_string(), relation.to_string());
    if seen_edges.insert(key) {
        edges.push(json!({
            "source": source,
            "target": target,
            "relation": relation,
        }));
    }
}

pub(super) fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(super) fn hot_context_range(line: usize) -> (usize, usize) {
    (
        line.saturating_sub(HOT_CONTEXT_BEFORE_LINES).max(1),
        line + HOT_CONTEXT_AFTER_LINES,
    )
}

pub(super) fn stable_node_id(kind: &str, value: &str) -> String {
    let mut rendered = String::with_capacity(kind.len() + value.len() + 1);
    rendered.push_str(kind);
    rendered.push(':');
    for character in value.chars() {
        if character == '_' || character == '-' || character == '/' || character == '.' {
            rendered.push(character);
        } else if character.is_ascii_alphanumeric() {
            rendered.push(character.to_ascii_lowercase());
        } else {
            rendered.push('-');
        }
    }
    while rendered.ends_with('-') {
        rendered.pop();
    }
    if rendered.len() == kind.len() + 1 {
        rendered.push_str("node");
    }
    rendered
}
