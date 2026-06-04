//! Projection metadata helpers for semantic query packets.

use std::collections::BTreeMap;

use serde_json::{Value, json};

pub(super) fn string_field_value(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn replace_item_patch_safety(
    item: &Value,
    exact_read: &str,
    source_fingerprint: &str,
) -> Option<Value> {
    let name = string_field_value(item, "name")?;
    let kind = string_field_value(item, "kind")?;
    if !rust_replace_item_kind_supported(&kind) {
        return None;
    }
    let (path, start_line, end_line) = parse_read_locator(exact_read)?;
    let locator = format!("{path}#{kind}:{name}");
    let line_range = format!("{start_line}:{end_line}");
    Some(json!({
        "level": "ast-patch-safe",
        "target": {
            "ownerPath": path.clone(),
            "locator": locator,
            "read": exact_read,
            "location": {
                "path": path,
                "lineRange": line_range,
            },
            "itemName": name.clone(),
            "itemKind": kind.clone(),
        },
        "preimageSource": "exact-read",
        "sourceFingerprint": source_fingerprint,
        "parserVersion": "rust:rs-harness",
        "allowedOperations": ["replace_item"],
        "losslessStructure": true,
        "notes": [
            "compact code is a save-token rustfmt-style projection; ast-patch apply must use exactRead",
            "Rust provider validates replacement with syn, reparses the file, runs rustfmt, then reparses formatted output"
        ],
    }))
}

fn rust_replace_item_kind_supported(kind: &str) -> bool {
    matches!(
        kind,
        "const"
            | "enum"
            | "fn"
            | "impl"
            | "mod"
            | "static"
            | "struct"
            | "trait"
            | "trait_alias"
            | "type"
            | "union"
    )
}

pub(super) fn projection_semantic_responsibilities(
    fields: &serde_json::Map<String, Value>,
    nodes: &[Value],
    exact_read: &str,
) -> Vec<Value> {
    let mut responsibilities = BTreeMap::<String, Value>::new();
    for kind in string_list_field(fields, "responsibilities") {
        insert_projection_responsibility(
            &mut responsibilities,
            kind,
            json!({
                "source": "native-parser",
                "read": exact_read,
            }),
        );
    }
    for node in nodes {
        for kind in projection_node_responsibility_kinds(node) {
            let mut evidence = json!({
                "source": "projection-node",
            });
            if let Some(node_id) = string_field_value(node, "id") {
                evidence["nodeId"] = json!(node_id);
            }
            if let Some(read) = string_field_value(node, "read") {
                evidence["read"] = json!(read);
            }
            insert_projection_responsibility(&mut responsibilities, kind, evidence);
        }
    }
    responsibilities.into_values().collect()
}

fn insert_projection_responsibility(
    responsibilities: &mut BTreeMap<String, Value>,
    kind: impl Into<String>,
    mut evidence: Value,
) {
    let kind = kind.into();
    evidence["kind"] = json!(kind.clone());
    responsibilities.entry(kind).or_insert(evidence);
}

fn projection_node_responsibility_kinds(node: &Value) -> Vec<&'static str> {
    let role = node["role"].as_str();
    let kind = node["kind"].as_str();
    match role {
        Some("mutation") => vec!["state-mutation"],
        Some("control-flow") if node_has_flag(node, "guard") => vec!["guard-branch"],
        Some("control-flow") if kind == Some("match") => vec!["match-dispatch"],
        Some("control-flow") if kind == Some("case") => vec!["match-arm"],
        Some("control-flow") if node_has_flag(node, "loop") && kind == Some("for") => {
            vec!["bounded-loop"]
        }
        Some("control-flow") if node_has_flag(node, "loop") => vec!["loop-control"],
        Some("terminal") if node_has_flag(node, "return") => vec!["early-return"],
        Some("call") => vec!["call-dispatch"],
        Some("effect") if node_has_flag(node, "await") => vec!["async-effect"],
        Some("effect") => vec!["effect-boundary"],
        Some("field") => vec!["data-shape"],
        _ => Vec::new(),
    }
}

fn node_has_flag(node: &Value, expected: &str) -> bool {
    node["flags"]
        .as_array()
        .is_some_and(|flags| flags.iter().any(|flag| flag.as_str() == Some(expected)))
}

pub(super) struct ProjectionNodeClassification {
    pub(super) kind: &'static str,
    pub(super) role: &'static str,
    pub(super) flags: &'static [&'static str],
}

pub(super) fn projection_node_classification(
    index: usize,
    label: &str,
) -> ProjectionNodeClassification {
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
    if label.starts_with("field ") {
        return ProjectionNodeClassification {
            kind: "field",
            role: "field",
            flags: &[],
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

fn parse_read_locator(value: &str) -> Option<(String, usize, usize)> {
    let (path, range) = value.rsplit_once(':')?;
    let (path, start) = path.rsplit_once(':')?;
    Some((path.to_string(), start.parse().ok()?, range.parse().ok()?))
}

fn string_list_field(fields: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    match fields.get(key) {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::String(value)) if !value.is_empty() => value
            .split(',')
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}
