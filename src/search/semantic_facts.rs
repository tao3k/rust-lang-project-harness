//! Provider-owned bounded semantic graph facts for ASP search pipe enrichment.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use serde_json::{Map, Value, json};

use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, ParsedRustModule, parse_cargo_dependency_facts,
    parse_cargo_manifest, parse_cargo_test_targets,
};

const HOT_CONTEXT_BEFORE_LINES: usize = 8;
const HOT_CONTEXT_AFTER_LINES: usize = 12;
const CANDIDATE_OWNER_LIMIT: usize = 16;
const PROJECT_SCAN_OWNER_LIMIT: usize = 256;
const PROJECT_SCAN_DIRECTORY_LIMIT: usize = 2048;
const FIELD_LIMIT: usize = 24;
const DEPENDENCY_LIMIT: usize = 32;
const TEST_TARGET_LIMIT: usize = 24;
const LANGUAGE_ID: &str = "rust";
const PROVIDER_ID: &str = "rs-harness";

/// Render bounded semantic graph facts for collection-field search enrichment.
pub fn render_rust_project_harness_search_semantic_facts_json(
    project_root: &Path,
    query: &str,
    input: &str,
) -> Result<String, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    let mut seen_edges = BTreeSet::new();
    emit_collection_field_graph_facts(
        query,
        semantic_fact_owners(project_root, input),
        &mut nodes,
        &mut edges,
        &mut seen_nodes,
        &mut seen_edges,
    );
    emit_cargo_project_graph_facts(
        project_root,
        &mut nodes,
        &mut edges,
        &mut seen_nodes,
        &mut seen_edges,
    );
    serde_json::to_string_pretty(&json!({
        "schemaId": "agent.semantic-protocols.semantic-fact-graph",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": LANGUAGE_ID,
        "providerId": PROVIDER_ID,
        "projectRoot": project_root.display().to_string().replace('\\', "/"),
        "query": query,
        "nodes": nodes,
        "edges": edges,
    }))
    .map(|mut text| {
        text.push('\n');
        text
    })
    .map_err(|error| format!("failed to render semantic fact JSON: {error}"))
}

fn emit_cargo_project_graph_facts(
    project_root: &Path,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let cargo_manifest = parse_cargo_manifest(project_root);
    let Some(package_name) = cargo_manifest.package_name.as_deref() else {
        return;
    };
    push_package_build_graph_facts(
        project_root,
        package_name,
        nodes,
        edges,
        seen_nodes,
        seen_edges,
    );
    push_package_bridge_edges(package_name, nodes, edges, seen_edges);
    parse_cargo_dependency_facts(project_root)
        .into_iter()
        .take(DEPENDENCY_LIMIT)
        .for_each(|dependency| {
            push_dependency_graph_facts(
                project_root,
                package_name,
                &dependency,
                nodes,
                edges,
                seen_nodes,
                seen_edges,
            );
        });
    parse_cargo_test_targets(project_root, &cargo_manifest)
        .into_iter()
        .filter(|test| test.report.is_valid)
        .take(TEST_TARGET_LIMIT)
        .for_each(|test| {
            push_test_target_graph_facts(
                project_root,
                package_name,
                &test,
                nodes,
                edges,
                seen_nodes,
                seen_edges,
            );
        });
}

fn push_package_build_graph_facts(
    project_root: &Path,
    package_name: &str,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let manifest_path = manifest_display_path(project_root);
    let package_id = package_node_id(package_name);
    let build_id = package_build_node_id(package_name);
    let build_command = cargo_test_command(package_name);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": package_id,
            "kind": "package",
            "role": "crate",
            "value": package_name,
            "action": "package",
            "path": manifest_path,
            "ownerPath": manifest_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{manifest_path}:1:1"),
            "matchText": package_name,
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "package",
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "packageName": package_name,
                "manifestPath": manifest_path,
            },
        }),
    );
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": build_id,
            "kind": "build",
            "role": "cargo-test",
            "value": build_command,
            "action": "build",
            "path": manifest_path,
            "ownerPath": manifest_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{manifest_path}:1:1"),
            "matchText": build_command,
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "build",
                "provenance": "build",
                "confidence": "exact",
                "freshness": "fresh",
                "packageName": package_name,
                "manifestPath": manifest_path,
                "tool": "cargo",
                "command": build_command,
            },
        }),
    );
    push_edge(edges, seen_edges, &package_id, &build_id, "builds");
}

fn push_package_bridge_edges(
    package_name: &str,
    nodes: &[Value],
    edges: &mut Vec<Value>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let package_id = package_node_id(package_name);
    for node in nodes {
        let Some(kind) = node.get("kind").and_then(Value::as_str) else {
            continue;
        };
        if !matches!(kind, "field" | "hot" | "owner") {
            continue;
        }
        let Some(node_id) = node.get("id").and_then(Value::as_str) else {
            continue;
        };
        push_edge(edges, seen_edges, node_id, &package_id, "belongs_to");
    }
}

fn push_dependency_graph_facts(
    project_root: &Path,
    package_name: &str,
    dependency: &CargoDependencyFacts,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let manifest_path = manifest_display_path(project_root);
    let package_id = package_node_id(package_name);
    let dependency_id = dependency_node_id(package_name, dependency);
    let dependency_kind = cargo_dependency_kind_label(dependency.kind);
    let fields = dependency_node_fields(package_name, &manifest_path, dependency, dependency_kind);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": dependency_id,
            "kind": "dependency",
            "role": dependency_kind,
            "value": dependency.package_name,
            "action": "deps",
            "path": manifest_path,
            "ownerPath": manifest_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{manifest_path}:1:1"),
            "matchText": dependency.package_name,
            "fields": fields,
        }),
    );
    push_edge(edges, seen_edges, &package_id, &dependency_id, "depends_on");
}

fn dependency_node_fields(
    package_name: &str,
    manifest_path: &str,
    dependency: &CargoDependencyFacts,
    dependency_kind: &str,
) -> Value {
    let mut fields = Map::new();
    fields.insert("languageId".to_string(), json!(LANGUAGE_ID));
    fields.insert("providerId".to_string(), json!(PROVIDER_ID));
    fields.insert("semanticFactKind".to_string(), json!("dependency"));
    fields.insert("provenance".to_string(), json!("parser"));
    fields.insert("confidence".to_string(), json!("exact"));
    fields.insert("freshness".to_string(), json!("fresh"));
    fields.insert("packageName".to_string(), json!(package_name));
    fields.insert("manifestPath".to_string(), json!(manifest_path));
    fields.insert(
        "dependencyKey".to_string(),
        json!(dependency.dependency_key),
    );
    fields.insert(
        "dependencyPackageName".to_string(),
        json!(dependency.package_name),
    );
    fields.insert("importName".to_string(), json!(dependency.import_name));
    fields.insert("dependencyKind".to_string(), json!(dependency_kind));
    fields.insert("optional".to_string(), json!(dependency.optional));
    fields.insert("features".to_string(), json!(dependency.features));
    if let Some(version_req) = dependency.version_req.as_deref() {
        fields.insert("versionReq".to_string(), json!(version_req));
    }
    if let Some(target) = dependency.target.as_deref() {
        fields.insert("target".to_string(), json!(target));
    }
    Value::Object(fields)
}

fn push_test_target_graph_facts(
    project_root: &Path,
    package_name: &str,
    test: &ParsedRustModule,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let package_id = package_node_id(package_name);
    let build_id = package_build_node_id(package_name);
    let test_path = display_project_path(project_root, &test.report.path);
    let test_name = test_target_name(&test_path);
    let test_id = test_target_node_id(package_name, &test_path);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": test_id,
            "kind": "test",
            "role": "cargo-test-target",
            "value": test_name,
            "action": "tests",
            "path": test_path,
            "ownerPath": test_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{test_path}:1:1"),
            "matchText": test_name,
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "semanticFactKind": "test",
                "provenance": "test",
                "confidence": "exact",
                "freshness": "fresh",
                "packageName": package_name,
                "testName": test_name,
                "testPath": test_path,
                "functionCount": test.syntax_facts.test_function_count,
                "command": cargo_test_command(package_name),
            },
        }),
    );
    push_edge(edges, seen_edges, &build_id, &test_id, "tests");
    push_edge(edges, seen_edges, &test_id, &package_id, "belongs_to");
}

fn emit_collection_field_graph_facts(
    query: &str,
    owners: Vec<CandidateOwner>,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let _ = owners
        .into_iter()
        .try_fold(FIELD_LIMIT, |remaining, owner| {
            if remaining == 0 {
                return ControlFlow::Break(());
            }
            let emitted = emit_owner_collection_fields(
                query, &owner, remaining, nodes, edges, seen_nodes, seen_edges,
            );
            let next_remaining = remaining.saturating_sub(emitted);
            if next_remaining == 0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(next_remaining)
            }
        });
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
    candidate_owners(project_root, input)
        .into_iter()
        .take(CANDIDATE_OWNER_LIMIT)
        .chain(project_collection_field_owners(project_root))
        .filter(|owner| seen.insert(owner.display.clone()))
        .collect()
}

fn project_collection_field_owners(project_root: &Path) -> Vec<CandidateOwner> {
    let mut files = project_scan_rust_files(project_root);
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

fn project_scan_rust_files(project_root: &Path) -> Vec<PathBuf> {
    let mut directories = vec![project_root.to_path_buf()];
    let mut visited_directories = 0usize;
    let mut files = Vec::new();
    while let Some(directory) =
        next_project_scan_directory(&mut directories, &mut visited_directories)
    {
        let Some(paths) = sorted_directory_paths(&directory) else {
            continue;
        };
        route_project_scan_paths(paths, &mut directories, &mut files);
    }
    files
}

fn next_project_scan_directory(
    directories: &mut Vec<PathBuf>,
    visited_directories: &mut usize,
) -> Option<PathBuf> {
    if *visited_directories >= PROJECT_SCAN_DIRECTORY_LIMIT {
        return None;
    }
    let directory = directories.pop()?;
    *visited_directories += 1;
    Some(directory)
}

fn sorted_directory_paths(directory: &Path) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(directory).ok()?;
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    Some(paths)
}

fn route_project_scan_paths(
    paths: Vec<PathBuf>,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    for path in paths.into_iter().rev() {
        route_project_scan_path(path, directories, files);
    }
}

fn route_project_scan_path(
    path: PathBuf,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) {
    if path.is_dir() {
        if !is_skipped_project_scan_directory(&path) {
            directories.push(path);
        }
    } else if path.extension().is_some_and(|extension| extension == "rs") {
        files.push(path);
    }
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
    let fields = collection_fields(&owner.display, syntax)
        .into_iter()
        .filter(|field| field.matches_query(query))
        .take(limit)
        .collect::<Vec<_>>();
    fields.iter().for_each(|field| {
        push_field_graph_facts(query, field, nodes, edges, seen_nodes, seen_edges);
    });
    fields.len()
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

fn package_node_id(package_name: &str) -> String {
    stable_node_id("package", package_name)
}

fn package_build_node_id(package_name: &str) -> String {
    stable_node_id("build", &cargo_test_command(package_name))
}

fn dependency_node_id(package_name: &str, dependency: &CargoDependencyFacts) -> String {
    stable_node_id(
        "dependency",
        &format!(
            "{}:{}:{}",
            package_name,
            cargo_dependency_kind_label(dependency.kind),
            dependency.package_name
        ),
    )
}

fn test_target_node_id(package_name: &str, test_path: &str) -> String {
    stable_node_id("test", &format!("{package_name}:{test_path}"))
}

fn cargo_dependency_kind_label(kind: CargoDependencyKind) -> &'static str {
    match kind {
        CargoDependencyKind::Normal => "normal",
        CargoDependencyKind::Dev => "dev",
        CargoDependencyKind::Build => "build",
    }
}

fn cargo_test_command(package_name: &str) -> String {
    format!("cargo test -p {package_name}")
}

fn manifest_display_path(project_root: &Path) -> String {
    display_project_path(project_root, &project_root.join("Cargo.toml"))
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn test_target_name(test_path: &str) -> String {
    Path::new(test_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(test_path)
        .to_string()
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
