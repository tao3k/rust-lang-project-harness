//! Collection semantic fact JSON nodes and edges.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::field_extract::CollectionField;
use crate::search::semantic_facts::contract::{LANGUAGE_ID, PROVIDER_ID};
use crate::search::semantic_facts::graph_helpers::{
    hot_context_range, push_edge, push_node, stable_node_id,
};

pub(super) fn push_field_graph_facts(
    query: &str,
    field: &CollectionField,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let query_id = stable_node_id("query", query);
    let owner_id = stable_node_id("owner", &field.owner_path);
    let field_id = collection_field_node_id(field);
    let type_id = collection_field_type_node_id(field);
    let collection_id = stable_node_id("collection", &field.collection_kind);
    let hot_id = collection_field_hot_node_id(field);
    let (start_line, end_line) = hot_context_range(field.line);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": owner_id,
            "kind": "owner",
            "role": "path",
            "value": field.owner_path,
            "action": "owner",
            "path": field.owner_path,
            "ownerPath": field.owner_path,
        }),
    );
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": field_id,
            "kind": "field",
            "role": "struct-field",
            "value": format!("{}: {}", field.field_name, field.type_value),
            "action": "code",
            "path": field.owner_path,
            "ownerPath": field.owner_path,
            "symbol": field.field_name,
            "startLine": field.line,
            "endLine": field.line,
            "locator": format!("{}:{}:{}", field.owner_path, field.line, field.line),
            "matchText": format!("{}::{}: {}", field.container_name, field.field_name, field.type_value),
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "field",
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "collectionFamily": collection_family(&field.collection_kind),
                "collectionImpl": field.collection_kind,
                "containerName": field.container_name,
                "fieldName": field.field_name,
                "typeName": field.collection_kind,
                "typeValue": field.type_value,
                "typeArgs": field.type_args,
                "collectionKind": field.collection_kind,
                "elementShape": field.element_shape,
                "contextStartLine": start_line,
                "contextEndLine": end_line,
                "contextLocator": format!("{}:{}:{}", field.owner_path, start_line, end_line),
                "field": field_fact(field),
            },
        }),
    );
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": type_id,
            "kind": "type",
            "role": "field-type",
            "value": field.type_value,
            "action": "evidence",
            "path": field.owner_path,
            "ownerPath": field.owner_path,
            "symbol": field.collection_kind,
            "startLine": field.line,
            "endLine": field.line,
            "locator": format!("{}:{}:{}", field.owner_path, field.line, field.line),
            "matchText": field.type_value,
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "type",
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "collectionFamily": collection_family(&field.collection_kind),
                "collectionImpl": field.collection_kind,
                "containerName": field.container_name,
                "fieldName": field.field_name,
                "typeName": field.collection_kind,
                "typeValue": field.type_value,
                "typeArgs": field.type_args,
                "collectionKind": field.collection_kind,
                "elementShape": field.element_shape,
                "type": type_fact(field),
            },
        }),
    );
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": collection_id,
            "kind": "collection",
            "role": "family",
            "value": field.collection_kind,
            "action": "evidence",
            "symbol": field.collection_kind,
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "collection",
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "collectionFamily": collection_family(&field.collection_kind),
                "collectionImpl": field.collection_kind,
                "collectionKind": field.collection_kind,
                "collection": collection_fact(field),
            },
        }),
    );
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": hot_id,
            "kind": "hot",
            "role": "field-range",
            "value": field.field_name,
            "action": "code",
            "path": field.owner_path,
            "ownerPath": field.owner_path,
            "symbol": field.field_name,
            "startLine": start_line,
            "endLine": end_line,
            "locator": format!("{}:{}:{}", field.owner_path, start_line, end_line),
            "matchText": format!("{}::{}: {}", field.container_name, field.field_name, field.type_value),
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "hot",
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "collectionFamily": collection_family(&field.collection_kind),
                "collectionImpl": field.collection_kind,
                "containerName": field.container_name,
                "fieldName": field.field_name,
                "typeName": field.collection_kind,
                "typeValue": field.type_value,
                "typeArgs": field.type_args,
                "collectionKind": field.collection_kind,
                "elementShape": field.element_shape,
            },
        }),
    );
    push_edge(edges, seen_edges, &query_id, &field_id, "matches");
    push_edge(edges, seen_edges, &query_id, &type_id, "matches");
    push_edge(edges, seen_edges, &query_id, &collection_id, "matches");
    push_edge(edges, seen_edges, &owner_id, &field_id, "contains");
    push_edge(edges, seen_edges, &field_id, &type_id, "has_type");
    push_edge(
        edges,
        seen_edges,
        &field_id,
        &collection_id,
        "collection_of",
    );
    push_edge(edges, seen_edges, &type_id, &collection_id, "collection_of");
    push_edge(edges, seen_edges, &field_id, &hot_id, "contains");
}

fn field_fact(field: &CollectionField) -> Value {
    json!({
        "ownerKind": "struct",
        "name": field.field_name,
        "ownerPath": field.owner_path,
        "access": field_access_modes(field),
    })
}

fn type_fact(field: &CollectionField) -> Value {
    let mut fact = json!({
        "name": field.type_value,
    });
    let args = collection_type_args(field);
    if let Some(object) = fact.as_object_mut() {
        match collection_family(&field.collection_kind) {
            "map" => {
                if let Some(key) = args.first() {
                    object.insert("key".to_string(), json!(key));
                }
                if let Some(value) = args.get(1) {
                    object.insert("value".to_string(), json!(value));
                }
            }
            _ => {
                if let Some(element) = args.first() {
                    object.insert("element".to_string(), json!(element));
                }
            }
        }
    }
    fact
}

fn collection_fact(field: &CollectionField) -> Value {
    let mut fact = json!({
        "family": collection_family(&field.collection_kind),
        "impl": field.collection_kind,
        "mutation": collection_mutation_modes(field),
    });
    let args = collection_type_args(field);
    if let Some(object) = fact.as_object_mut() {
        match collection_family(&field.collection_kind) {
            "map" => {
                if let Some(key) = args.first() {
                    object.insert("keyType".to_string(), json!(key));
                }
                if let Some(value) = args.get(1) {
                    object.insert("valueType".to_string(), json!(value));
                }
            }
            _ => {
                if let Some(element) = args.first() {
                    object.insert("elementType".to_string(), json!(element));
                }
            }
        }
    }
    fact
}

fn collection_family(collection_kind: &str) -> &'static str {
    match collection_kind {
        "Vec" | "VecDeque" => "sequence",
        "HashMap" | "BTreeMap" => "map",
        "HashSet" | "BTreeSet" => "set",
        _ => "iterator",
    }
}

fn field_access_modes(field: &CollectionField) -> Vec<&'static str> {
    match collection_family(&field.collection_kind) {
        "map" => vec!["read", "write", "validate"],
        "set" => vec!["read", "append", "validate"],
        _ => vec!["read", "append", "validate"],
    }
}

fn collection_mutation_modes(field: &CollectionField) -> Vec<&'static str> {
    match collection_family(&field.collection_kind) {
        "map" => vec!["insert", "remove", "update"],
        "set" => vec!["insert", "remove"],
        _ => vec!["append", "remove"],
    }
}

fn collection_type_args(field: &CollectionField) -> Vec<String> {
    field
        .type_args
        .split(", ")
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn collection_field_node_id(field: &CollectionField) -> String {
    stable_node_id(
        "field",
        &format!("{}:{}:{}", field.owner_path, field.field_name, field.line),
    )
}

fn collection_field_type_node_id(field: &CollectionField) -> String {
    stable_node_id(
        "type",
        &format!(
            "{}:{}:{}:{}",
            field.owner_path, field.field_name, field.type_value, field.line
        ),
    )
}

fn collection_field_hot_node_id(field: &CollectionField) -> String {
    stable_node_id(
        "hot",
        &format!("{}:{}:{}", field.owner_path, field.field_name, field.line),
    )
}
