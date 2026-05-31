use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_agent_registry_advertises_package_local_semantic_schemas() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();

    let registry = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(registry.status.success(), "{registry:?}");
    let value = serde_json::from_slice::<Value>(&registry.stdout).expect("agent registry json");
    let schemas = value["languages"][0]["schemas"]
        .as_array()
        .expect("schemas");

    for expected in semantic_schema_files() {
        let advertised = schemas
            .iter()
            .find(|schema| schema["schemaId"].as_str() == Some(expected.schema_id))
            .unwrap_or_else(|| panic!("missing advertised schema: {}", expected.schema_id));
        assert_eq!(advertised["schemaVersion"], "1");
        assert_eq!(advertised["path"], expected.registry_path);

        let schema_path = package_root().join(expected.registry_path);
        assert!(
            schema_path.exists(),
            "advertised schema path does not exist: {}",
            schema_path.display()
        );
        let schema = read_json(&schema_path);
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(
            schema["$id"]
                .as_str()
                .is_some_and(|id| id.ends_with(expected.file_name)),
            "{schema}"
        );
        assert_eq!(
            schema_pointer(&schema, expected.identity_pointer),
            Some(expected.schema_id),
            "{schema}"
        );
    }
}

#[test]
fn package_local_semantic_schemas_match_protocol_repository_when_present() {
    for expected in semantic_schema_files() {
        let Some(protocol_schema_path) = protocol_repository_schema_path(expected.file_name) else {
            continue;
        };
        let package_schema_path = package_root().join(expected.registry_path);
        assert_eq!(
            read_json(&package_schema_path),
            read_json(&protocol_schema_path),
            "{} matches the protocol repository schema",
            expected.file_name
        );
    }
}

struct SemanticSchemaFile {
    schema_id: &'static str,
    file_name: &'static str,
    registry_path: &'static str,
    identity_pointer: &'static [&'static str],
}

fn semantic_schema_files() -> &'static [SemanticSchemaFile] {
    &[
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-language-registry",
            file_name: "semantic-language-registry.v1.schema.json",
            registry_path: "schemas/semantic-language-registry.v1.schema.json",
            identity_pointer: &["properties", "registryId", "const"],
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-search-packet",
            file_name: "semantic-search-packet.v1.schema.json",
            registry_path: "schemas/semantic-search-packet.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
        },
    ]
}

fn package_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn protocol_repository_schema_path(file_name: &str) -> Option<PathBuf> {
    let path = package_root().join("../..").join("schemas").join(file_name);
    path.exists().then_some(path)
}

fn read_json(path: &Path) -> Value {
    let content =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|error| panic!("parse {} as JSON: {error}", path.display()))
}

fn schema_pointer<'a>(schema: &'a Value, pointer: &[&str]) -> Option<&'a str> {
    pointer
        .iter()
        .try_fold(schema, |value, key| value.get(*key))
        .and_then(Value::as_str)
}
