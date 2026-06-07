//! Provider-owned bounded semantic graph facts for ASP search pipe enrichment.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use serde_json::{Value, json};

const HOT_CONTEXT_BEFORE_LINES: usize = 8;
const HOT_CONTEXT_AFTER_LINES: usize = 12;
const CANDIDATE_OWNER_LIMIT: usize = 16;
const PROJECT_SCAN_OWNER_LIMIT: usize = 256;
const PROJECT_SCAN_DIRECTORY_LIMIT: usize = 2048;
const FIELD_LIMIT: usize = 24;

pub fn render_rust_project_harness_search_semantic_facts_json(
    project_root: &Path,
    query: &str,
    input: &str,
) -> Result<String, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    let mut seen_edges = BTreeSet::new();
    let mut emitted_fields = 0usize;
    for owner in semantic_fact_owners(project_root, input) {
        if emitted_fields >= FIELD_LIMIT {
            break;
        }
        emitted_fields += emit_owner_collection_fields(
            query,
            &owner,
            FIELD_LIMIT.saturating_sub(emitted_fields),
            &mut nodes,
            &mut edges,
            &mut seen_nodes,
            &mut seen_edges,
        );
    }
    serde_json::to_string_pretty(&json!({
        "schemaId": "agent.semantic-protocols.semantic-graph-fragment",
        "schemaVersion": "1",
        "nodes": nodes,
        "edges": edges,
    }))
    .map(|mut text| {
        text.push('\n');
        text
    })
    .map_err(|error| format!("failed to render semantic fact JSON: {error}"))
}

#[derive(Debug, Clone)]
struct CandidateOwner {
    display: String,
    absolute: PathBuf,
}

#[derive(Debug, Clone)]
struct CollectionField {
    owner_path: String,
    container_name: String,
    field_name: String,
    type_value: String,
    type_args: String,
    collection_kind: String,
    element_shape: String,
    line: usize,
}

impl CollectionField {
    fn matches_query(&self, query: &str) -> bool {
        let terms = query_terms(query);
        if terms
            .iter()
            .any(|term| collection_term_constrains_kind(term))
            && !terms
                .iter()
                .any(|term| collection_term_matches_kind(term, &self.collection_kind))
        {
            return false;
        }
        let text = format!(
            "{} {} {} {} {} {} field fields type types collection collections list lists map maps set sets",
            self.container_name,
            self.field_name,
            self.type_value,
            self.type_args,
            self.collection_kind,
            self.element_shape,
        )
        .to_ascii_lowercase();
        terms
            .iter()
            .any(|term| text.contains(term.as_str()) || self.alias_matches(term))
    }

    fn alias_matches(&self, term: &str) -> bool {
        match term {
            "collection" | "collections" | "list" | "lists" => true,
            "map" | "maps" => self.collection_kind.ends_with("Map"),
            "set" | "sets" => self.collection_kind.ends_with("Set"),
            "field" | "fields" | "type" | "types" => true,
            "scalar" | "scalars" => self.element_shape == "scalar",
            _ => false,
        }
    }
}

fn collection_term_constrains_kind(term: &str) -> bool {
    matches!(
        term,
        "vec"
            | "vecdeque"
            | "hashmap"
            | "hashset"
            | "btreemap"
            | "btreeset"
            | "map"
            | "maps"
            | "set"
            | "sets"
            | "list"
            | "lists"
    )
}

fn collection_term_matches_kind(term: &str, collection_kind: &str) -> bool {
    let kind = collection_kind.to_ascii_lowercase();
    match term {
        "vec" => kind == "vec",
        "vecdeque" => kind == "vecdeque",
        "hashmap" => kind == "hashmap",
        "hashset" => kind == "hashset",
        "btreemap" => kind == "btreemap",
        "btreeset" => kind == "btreeset",
        "map" | "maps" => kind.ends_with("map"),
        "set" | "sets" => kind.ends_with("set"),
        "list" | "lists" => matches!(kind.as_str(), "vec" | "vecdeque"),
        _ => false,
    }
}

fn candidate_owners(project_root: &Path, input: &str) -> Vec<CandidateOwner> {
    let mut seen = HashSet::new();
    input
        .lines()
        .filter_map(|line| line.split_once(':').map(|(path, _)| path))
        .filter(|path| !path.trim().is_empty())
        .filter_map(|path| {
            let display = path.to_string();
            let absolute = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                project_root.join(path)
            };
            (absolute.exists() && seen.insert(display.clone()))
                .then_some(CandidateOwner { display, absolute })
        })
        .collect()
}

fn semantic_fact_owners(project_root: &Path, input: &str) -> Vec<CandidateOwner> {
    let mut seen = HashSet::new();
    let mut owners = Vec::new();
    for owner in candidate_owners(project_root, input)
        .into_iter()
        .take(CANDIDATE_OWNER_LIMIT)
    {
        if seen.insert(owner.display.clone()) {
            owners.push(owner);
        }
    }
    for owner in project_collection_field_owners(project_root) {
        if seen.insert(owner.display.clone()) {
            owners.push(owner);
        }
    }
    owners
}

fn project_collection_field_owners(project_root: &Path) -> Vec<CandidateOwner> {
    let mut directories = vec![project_root.to_path_buf()];
    let mut visited_directories = 0usize;
    let mut files = Vec::new();
    while let Some(directory) = directories.pop() {
        if visited_directories >= PROJECT_SCAN_DIRECTORY_LIMIT {
            break;
        }
        visited_directories += 1;
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths.into_iter().rev() {
            if path.is_dir() {
                if !is_skipped_project_scan_directory(&path) {
                    directories.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort_by_key(|path| (project_scan_file_priority(project_root, path), path.clone()));
    files
        .into_iter()
        .take(PROJECT_SCAN_OWNER_LIMIT)
        .filter_map(|absolute| {
            let display = absolute.strip_prefix(project_root).ok()?.to_string_lossy();
            Some(CandidateOwner {
                display: display.to_string(),
                absolute,
            })
        })
        .collect()
}

fn is_skipped_project_scan_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".cache" | "target" | "node_modules" | "vendor"
            )
        })
}

fn project_scan_file_priority(project_root: &Path, path: &Path) -> u8 {
    let relative = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy();
    let normalized = relative.replace('\\', "/");
    let is_test_like = normalized.contains("/tests/")
        || normalized.starts_with("tests/")
        || normalized.contains("/examples/")
        || normalized.starts_with("examples/")
        || normalized.contains("/benches/")
        || normalized.starts_with("benches/")
        || normalized.contains("stress-test/");
    if normalized.contains("/src/") && !is_test_like {
        0
    } else if normalized.contains("/src/") {
        1
    } else if !is_test_like {
        2
    } else {
        3
    }
}

fn emit_owner_collection_fields(
    query: &str,
    owner: &CandidateOwner,
    limit: usize,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) -> usize {
    let Ok(source) = fs::read_to_string(&owner.absolute) else {
        return 0;
    };
    let Ok(syntax) = syn::parse_file(&source) else {
        return 0;
    };
    emit_collection_fields_from_syntax(
        query, owner, &syntax, limit, nodes, edges, seen_nodes, seen_edges,
    )
}

fn emit_collection_fields_from_syntax(
    query: &str,
    owner: &CandidateOwner,
    syntax: &syn::File,
    limit: usize,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) -> usize {
    let mut emitted = 0usize;
    for field in collection_fields(&owner.display, syntax)
        .into_iter()
        .filter(|field| field.matches_query(query))
        .take(limit)
    {
        push_field_graph_facts(query, &field, nodes, edges, seen_nodes, seen_edges);
        emitted += 1;
    }
    emitted
}

fn collection_fields(owner_path: &str, syntax: &syn::File) -> Vec<CollectionField> {
    syntax
        .items
        .iter()
        .flat_map(|item| match item {
            syn::Item::Struct(item_struct) => struct_collection_fields(owner_path, item_struct),
            _ => Vec::new(),
        })
        .collect()
}

fn struct_collection_fields(
    owner_path: &str,
    item_struct: &syn::ItemStruct,
) -> Vec<CollectionField> {
    let syn::Fields::Named(fields) = &item_struct.fields else {
        return Vec::new();
    };
    let container_name = item_struct.ident.to_string();
    fields
        .named
        .iter()
        .filter_map(|field| {
            let field_name = field.ident.as_ref()?.to_string();
            let collection = direct_collection_type(&field.ty)?;
            Some(CollectionField {
                owner_path: owner_path.to_string(),
                container_name: container_name.clone(),
                field_name,
                type_value: field.ty.to_token_stream().to_string(),
                type_args: collection.type_args.clone(),
                collection_kind: collection.kind.clone(),
                element_shape: collection.element_shape(),
                line: field.ident.as_ref()?.span().start().line.max(1),
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct CollectionType {
    kind: String,
    type_args: String,
}

impl CollectionType {
    fn element_shape(&self) -> String {
        match self.kind.as_str() {
            "Vec" | "VecDeque" => {
                if self.type_args.trim().is_empty() {
                    "unknown"
                } else if contains_collection_type(&self.type_args) {
                    "collection"
                } else {
                    "scalar"
                }
            }
            "HashMap" | "BTreeMap" => "key-value",
            "HashSet" | "BTreeSet" => "scalar",
            _ => "unknown",
        }
        .to_string()
    }
}

fn contains_collection_type(value: &str) -> bool {
    [
        "Vec", "VecDeque", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    ]
    .iter()
    .any(|name| value.contains(name))
}

fn direct_collection_type(ty: &syn::Type) -> Option<CollectionType> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let kind = segment.ident.to_string();
    if !matches!(
        kind.as_str(),
        "Vec" | "VecDeque" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
    ) {
        return None;
    }
    Some(CollectionType {
        kind,
        type_args: generic_args_text(&segment.arguments),
    })
}

fn generic_args_text(arguments: &syn::PathArguments) -> String {
    let syn::PathArguments::AngleBracketed(arguments) = arguments else {
        return String::new();
    };
    arguments
        .args
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|tokens| tokens.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn push_field_graph_facts(
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
                "collectionKind": field.collection_kind,
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

fn push_node(nodes: &mut Vec<Value>, seen_nodes: &mut BTreeSet<String>, node: Value) {
    let Some(id) = node.get("id").and_then(Value::as_str) else {
        return;
    };
    if seen_nodes.insert(id.to_string()) {
        nodes.push(node);
    }
}

fn push_edge(
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

fn hot_context_range(line: usize) -> (usize, usize) {
    (
        line.saturating_sub(HOT_CONTEXT_BEFORE_LINES).max(1),
        line + HOT_CONTEXT_AFTER_LINES,
    )
}

fn stable_node_id(kind: &str, value: &str) -> String {
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

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !(character == '_' || character.is_ascii_alphanumeric()))
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value as JsonValue;

    use super::*;

    #[test]
    fn semantic_facts_project_scan_finds_collection_fields_without_candidate_owner() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join("src")).expect("create src");
        fs::write(
            tempdir.path().join("src/model.rs"),
            "pub struct Scalar;\n\
             pub struct Snapshot {\n\
                 pub scalars: Vec<Scalar>,\n\
                 pub nested: Vec<Vec<u8>>,\n\
             }\n",
        )
        .expect("write model");
        fs::write(tempdir.path().join("src/lexical.rs"), "fn vec_hit() {}\n")
            .expect("write lexical");

        let rendered = render_rust_project_harness_search_semantic_facts_json(
            tempdir.path(),
            "Vec scalar collection fields",
            "src/lexical.rs:1:1:Vec\n",
        )
        .expect("render facts");
        let packet: JsonValue = serde_json::from_str(&rendered).expect("json");
        let nodes = packet["nodes"].as_array().expect("nodes");
        let edges = packet["edges"].as_array().expect("edges");

        assert!(nodes.iter().any(|node| {
            node["kind"].as_str() == Some("owner") && node["value"].as_str() == Some("src/model.rs")
        }));
        assert!(nodes.iter().any(|node| {
            node["kind"].as_str() == Some("field")
                && node["symbol"].as_str() == Some("scalars")
                && node["fields"]["typeValue"].as_str() == Some("Vec < Scalar >")
                && node["fields"]["elementShape"].as_str() == Some("scalar")
                && node["fields"]["contextLocator"].as_str() == Some("src/model.rs:1:15")
        }));
        assert!(nodes.iter().any(|node| {
            node["kind"].as_str() == Some("type")
                && node["fields"]["fieldName"].as_str() == Some("scalars")
        }));
        assert!(
            edges
                .iter()
                .any(|edge| edge["relation"].as_str() == Some("has_type"))
        );
    }
}
