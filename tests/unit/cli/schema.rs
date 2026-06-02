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
        if !expected.syncs_with_protocol_repository {
            continue;
        }
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

#[test]
fn cli_agent_registry_uses_rust_capability_vocabulary() {
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
    let methods = value["languages"][0]["methodDescriptors"]
        .as_array()
        .expect("method descriptors");
    assert!(
        methods
            .iter()
            .all(|descriptor| descriptor["method"] != "search/text"),
        "{methods:?}"
    );
    let rust_capability_schema =
        read_json(&package_root().join("schemas/rust-semantic-capabilities.v1.schema.json"));
    let capability_names = schema_enum(
        &rust_capability_schema,
        &[
            "$defs",
            "capabilityDescriptor",
            "properties",
            "name",
            "enum",
        ],
    );
    let ingest_surface_names = schema_enum(
        &rust_capability_schema,
        &[
            "$defs",
            "ingestSurfaceDescriptor",
            "properties",
            "name",
            "enum",
        ],
    );

    let deps = method_descriptor(methods, "search/deps");
    assert!(
        deps["capabilities"].as_array().is_some_and(|capabilities| {
            capabilities.iter().any(|capability| {
                capability["namespace"] == "rust"
                    && capability["name"] == "dependency-api-token-usage-search"
            })
        }),
        "{deps}"
    );
    let dependency = method_descriptor(methods, "search/dependency");
    assert_eq!(dependency["supportsQuerySet"], true);
    assert_eq!(
        dependency["acceptedQuerySetSelectors"],
        serde_json::json!(["exact-set"])
    );
    let owner = method_descriptor(methods, "search/owner");
    assert_eq!(owner["supportsQuerySet"], true);
    assert_eq!(
        owner["acceptedQuerySetSelectors"],
        serde_json::json!(["exact-set"])
    );
    assert_eq!(
        owner["acceptedPipes"],
        serde_json::json!(["items", "tests"])
    );
    let fzf = method_descriptor(methods, "search/fzf");
    assert_eq!(fzf["requiresQuery"], true);
    assert_eq!(fzf["supportsQuerySet"], true);
    assert_eq!(
        fzf["acceptedQuerySetSelectors"],
        serde_json::json!(["fuzzy-set"])
    );
    assert!(
        fzf["capabilities"].as_array().is_some_and(|capabilities| {
            capabilities.iter().any(|capability| {
                capability["namespace"] == "semantic"
                    && capability["name"] == "finder-fuzzy-candidate-search"
            }) && capabilities.iter().any(|capability| {
                capability["namespace"] == "rust"
                    && capability["name"] == "parser-visible-source-fuzzy-search"
            })
        }),
        "{fzf}"
    );
    let policy = method_descriptor(methods, "search/policy");
    assert_eq!(
        policy["outputSchemaIds"],
        serde_json::json!([
            "agent.semantic-protocols.semantic-search-packet",
            "agent.semantic-protocols.semantic-handle"
        ])
    );
    assert_eq!(
        policy["acceptedPipes"],
        serde_json::json!(["owner", "tests"])
    );
    assert!(
        policy["capabilities"]
            .as_array()
            .is_some_and(|capabilities| {
                capabilities.iter().any(|capability| {
                    capability["namespace"] == "rust"
                        && capability["name"] == "rust-project-policy-rule-handle-search"
                })
            }),
        "{policy}"
    );
    let query_owner_items = method_descriptor(methods, "query/owner-items");
    assert_eq!(query_owner_items["command"], "query");
    assert_eq!(query_owner_items["input"], "owner-path");
    assert_eq!(
        query_owner_items["requiredOptions"],
        serde_json::json!(["--term"])
    );
    assert_eq!(
        query_owner_items["outputSchemaIds"],
        serde_json::json!(["agent.semantic-protocols.semantic-query-packet"])
    );
    assert_eq!(
        query_owner_items["outputModes"],
        serde_json::json!(["compact", "json", "code", "names"])
    );
    assert_eq!(query_owner_items["supportsQuerySet"], true);
    assert_eq!(
        query_owner_items["acceptedQuerySetSelectors"],
        serde_json::json!(["exact-set"])
    );
    assert_eq!(
        query_owner_items["querySetScopes"],
        serde_json::json!(["owner"])
    );
    let direct_source_read = method_descriptor(methods, "query/direct-source-read");
    assert_eq!(direct_source_read["command"], "query");
    assert_eq!(direct_source_read["input"], "owner-path");
    assert_eq!(
        direct_source_read["requiredOptions"],
        serde_json::json!(["--from-hook", "--selector"])
    );
    assert_eq!(
        direct_source_read["outputSchemaIds"],
        serde_json::json!([
            "agent.semantic-protocols.semantic-query-packet",
            "agent.semantic-protocols.semantic-read-packet"
        ])
    );
    assert_eq!(
        direct_source_read["outputModes"],
        serde_json::json!(["compact", "json", "names", "read-packet"])
    );
    let fzf = method_descriptor(methods, "search/fzf");
    assert_eq!(fzf["supportsQuerySet"], true);
    assert_eq!(
        fzf["acceptedQuerySetSelectors"],
        serde_json::json!(["fuzzy-set"])
    );
    let public_external_types = method_descriptor(methods, "search/public-external-types");
    assert_eq!(
        public_external_types["outputSchemaIds"],
        serde_json::json!([
            "agent.semantic-protocols.semantic-search-packet",
            "agent.semantic-protocols.semantic-type-surface"
        ])
    );
    let tests = method_descriptor(methods, "search/tests");
    assert_eq!(tests["supportsQuerySet"], true);
    assert_eq!(
        tests["acceptedQuerySetSelectors"],
        serde_json::json!(["exact-set"])
    );
    let ingest = method_descriptor(methods, "search/ingest");
    assert_eq!(
        ingest["acceptedPipes"],
        serde_json::json!(["items", "tests"])
    );
    assert!(
        fzf["ingestRequiredFor"].as_array().is_some_and(|surfaces| {
            surfaces
                .iter()
                .any(|surface| surface["name"] == "schema-json")
        }),
        "{fzf}"
    );

    for descriptor in methods {
        for capability in descriptor["capabilities"].as_array().into_iter().flatten() {
            assert_eq!(capability["languageId"], "rust", "{capability}");
            let name = capability["name"].as_str().expect("capability name");
            assert!(
                capability_names.iter().any(|candidate| candidate == name),
                "unknown capability {name}: {capability_names:?}"
            );
        }
        for surface in descriptor["ingestRequiredFor"]
            .as_array()
            .into_iter()
            .flatten()
        {
            assert_eq!(surface["languageId"], "rust", "{surface}");
            let name = surface["name"].as_str().expect("surface name");
            assert!(
                ingest_surface_names
                    .iter()
                    .any(|candidate| candidate == name),
                "unknown ingest surface {name}: {ingest_surface_names:?}"
            );
        }
    }
}

struct SemanticSchemaFile {
    schema_id: &'static str,
    file_name: &'static str,
    registry_path: &'static str,
    identity_pointer: &'static [&'static str],
    syncs_with_protocol_repository: bool,
}

fn semantic_schema_files() -> &'static [SemanticSchemaFile] {
    &[
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-language-registry",
            file_name: "semantic-language-registry.v1.schema.json",
            registry_path: "schemas/semantic-language-registry.v1.schema.json",
            identity_pointer: &["properties", "registryId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-search-packet",
            file_name: "semantic-search-packet.v1.schema.json",
            registry_path: "schemas/semantic-search-packet.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-query-packet",
            file_name: "semantic-query-packet.v1.schema.json",
            registry_path: "schemas/semantic-query-packet.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-read-packet",
            file_name: "semantic-read-packet.v1.schema.json",
            registry_path: "schemas/semantic-read-packet.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-graph",
            file_name: "semantic-graph.v1.schema.json",
            registry_path: "schemas/semantic-graph.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-type-surface",
            file_name: "semantic-type-surface.v1.schema.json",
            registry_path: "schemas/semantic-type-surface.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-handle",
            file_name: "semantic-handle.v1.schema.json",
            registry_path: "schemas/semantic-handle.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-native-syntax-fact-index",
            file_name: "semantic-native-syntax-fact-index.v1.schema.json",
            registry_path: "schemas/semantic-native-syntax-fact-index.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.semantic-review-packet",
            file_name: "semantic-review-packet.v1.schema.json",
            registry_path: "schemas/semantic-review-packet.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: true,
        },
        SemanticSchemaFile {
            schema_id: "agent.semantic-protocols.languages.rust.rs-harness.capabilities",
            file_name: "rust-semantic-capabilities.v1.schema.json",
            registry_path: "schemas/rust-semantic-capabilities.v1.schema.json",
            identity_pointer: &["properties", "schemaId", "const"],
            syncs_with_protocol_repository: false,
        },
    ]
}

fn package_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn protocol_repository_schema_path(file_name: &str) -> Option<PathBuf> {
    protocol_repository_schema_paths(file_name)
        .into_iter()
        .find(|path| path.exists())
}

fn protocol_repository_schema_paths(file_name: &str) -> Vec<PathBuf> {
    let mut paths = vec![package_root().join("../..").join("schemas").join(file_name)];
    if let Some(owner_root) = package_root().parent() {
        paths.push(
            owner_root
                .join("agent-semantic-protocols")
                .join("schemas")
                .join(file_name),
        );
    }
    paths
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

fn schema_enum(schema: &Value, pointer: &[&str]) -> Vec<String> {
    pointer
        .iter()
        .try_fold(schema, |value, key| value.get(*key))
        .and_then(Value::as_array)
        .expect("schema enum")
        .iter()
        .map(|value| value.as_str().expect("enum string").to_string())
        .collect()
}

fn method_descriptor<'a>(methods: &'a [Value], method: &str) -> &'a Value {
    methods
        .iter()
        .find(|descriptor| descriptor["method"].as_str() == Some(method))
        .unwrap_or_else(|| panic!("missing method descriptor: {method}"))
}
