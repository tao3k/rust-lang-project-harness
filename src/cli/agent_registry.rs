//! Semantic-language registry JSON for agent capability discovery.

use std::path::Path;

use serde_json::{Value, json};

pub(super) fn print_agent_registry(project_root: &Path) -> Result<(), String> {
    let registry = agent_registry_json(project_root);
    println!(
        "{}",
        serde_json::to_string(&registry)
            .map_err(|error| format!("failed to render agent registry JSON: {error}"))?
    );
    Ok(())
}

fn agent_registry_json(project_root: &Path) -> Value {
    let search_methods = [
        "workspace",
        "prime",
        "owner",
        "dependency",
        "deps",
        "features",
        "targets",
        "symbol",
        "callsite",
        "import",
        "tests",
        "text",
        "cfg",
        "patterns",
        "pattern",
        "docs",
        "docs-use",
        "api",
        "public-external-types",
        "ingest",
    ];
    let mut methods = search_methods
        .iter()
        .map(|view| format!("search/{view}"))
        .chain([
            "check/changed".to_string(),
            "check/full".to_string(),
            "agent/install".to_string(),
            "agent/doctor".to_string(),
        ])
        .collect::<Vec<_>>();
    methods.sort();

    let mut method_descriptors = search_methods
        .iter()
        .map(|view| search_method_descriptor(view))
        .collect::<Vec<_>>();
    method_descriptors.extend([
        json!({
            "method": "check/changed",
            "command": "check",
            "supportsJson": true,
            "supportsCompact": true
        }),
        json!({
            "method": "check/full",
            "command": "check",
            "supportsJson": true,
            "supportsCompact": true
        }),
        json!({
            "method": "agent/install",
            "command": "agent",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-language-registry"],
            "supportsJson": true,
            "supportsCompact": true
        }),
        json!({
            "method": "agent/doctor",
            "command": "agent",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-language-registry"],
            "supportsJson": true,
            "supportsCompact": true
        }),
    ]);

    json!({
        "registryId": "agent.semantic-protocols.semantic-language-registry",
        "registryVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "projectRoot": display_absolute_cli_path(project_root),
        "languages": [
            {
                "languageId": "rust",
                "providerId": "rs-harness",
                "binary": "rs-harness",
                "namespace": "agent.semantic-protocols.languages.rust.rs-harness",
                "displayName": "Rust Harness",
                "methods": methods,
                "methodDescriptors": method_descriptors,
                "schemas": [
                    {
                        "schemaId": "agent.semantic-protocols.semantic-language-registry",
                        "schemaVersion": "1",
                        "path": "schemas/semantic-language-registry.v1.schema.json"
                    },
                    {
                        "schemaId": "agent.semantic-protocols.semantic-search-packet",
                        "schemaVersion": "1",
                        "path": "schemas/semantic-search-packet.v1.schema.json"
                    }
                ]
            }
        ]
    })
}

fn search_method_descriptor(view: &str) -> Value {
    json!({
        "method": format!("search/{view}"),
        "command": "search",
        "view": view,
        "outputSchemaIds": ["agent.semantic-protocols.semantic-search-packet"],
        "requiresQuery": search_view_requires_query(view),
        "acceptsStdin": view == "ingest",
        "supportsPackageScope": true,
        "supportsJson": true,
        "supportsCompact": true
    })
}

fn search_view_requires_query(view: &str) -> bool {
    matches!(
        view,
        "owner"
            | "dependency"
            | "tests"
            | "symbol"
            | "callsite"
            | "import"
            | "text"
            | "cfg"
            | "pattern"
            | "docs"
            | "docs-use"
            | "api"
    )
}

fn display_absolute_cli_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
