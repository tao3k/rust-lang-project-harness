//! Cargo package, dependency, and test semantic graph facts.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, ParsedRustModule, parse_cargo_dependency_facts,
    parse_cargo_manifest, parse_cargo_test_targets,
};

use super::contract::{LANGUAGE_ID, PROVIDER_ID};
use super::graph_helpers::{display_project_path, push_edge, push_node, stable_node_id};

const DEPENDENCY_LIMIT: usize = 32;
const TEST_TARGET_LIMIT: usize = 24;

pub(super) fn emit_cargo_project_graph_facts(
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

fn test_target_name(test_path: &str) -> String {
    Path::new(test_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(test_path)
        .to_string()
}
