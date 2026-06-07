use serde_json::Value;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_agent_registry_advertises_relation_flow_codeql_schemas_without_codeql_backend() {
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
    let language = &value["languages"][0];
    let schemas = language["schemas"].as_array().expect("schemas");

    for schema_id in [
        "agent.semantic-protocols.semantic-relation-plan",
        "agent.semantic-protocols.semantic-flow-lite",
        "agent.semantic-protocols.semantic-codeql-evidence",
    ] {
        assert!(
            schemas
                .iter()
                .any(|schema| schema["schemaId"].as_str() == Some(schema_id)),
            "missing structured relation/flow schema {schema_id}: {schemas:?}"
        );
    }

    let methods = language["methodDescriptors"]
        .as_array()
        .expect("method descriptors");
    for descriptor in methods {
        let Some(backends) = descriptor["executionBackends"].as_array() else {
            continue;
        };
        assert!(
            !backends.iter().any(|backend| backend == "codeql"),
            "Rust provider should keep CodeQL unavailable until an executor exists: {descriptor}"
        );
    }
}
