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
            "agent/guide".to_string(),
            "agent/hook".to_string(),
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
            "clients": ["codex"],
            "requiredOptions": ["--client codex"],
            "outputSchemaIds": ["agent.semantic-protocols.semantic-language-registry"],
            "supportsJson": true,
            "supportsCompact": true
        }),
        json!({
            "method": "agent/doctor",
            "command": "agent",
            "clients": ["codex"],
            "outputSchemaIds": ["agent.semantic-protocols.semantic-language-registry"],
            "supportsJson": true,
            "supportsCompact": true
        }),
        json!({
            "method": "agent/guide",
            "command": "agent",
            "clients": ["codex"],
            "requiredOptions": ["--client codex"],
            "supportsJson": false,
            "supportsCompact": true,
            "capabilities": [
                semantic_capability("agent-hook-policy"),
                rust_capability("harness-search-checkpoints")
            ]
        }),
        json!({
            "method": "agent/hook",
            "command": "agent",
            "clients": ["codex"],
            "requiredOptions": ["--client codex"],
            "input": "hook event JSON on stdin",
            "supportsJson": true,
            "supportsCompact": false,
            "capabilities": [
                semantic_capability("agent-hook-policy"),
                rust_capability("harness-search-checkpoints")
            ]
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
                    },
                    {
                        "schemaId": "agent.semantic-protocols.languages.rust.rs-harness.capabilities",
                        "schemaVersion": "1",
                        "path": "schemas/rust-semantic-capabilities.v1.schema.json"
                    }
                ]
            }
        ]
    })
}

fn search_method_descriptor(view: &str) -> Value {
    let mut descriptor = json!({
        "method": format!("search/{view}"),
        "command": "search",
        "view": view,
        "outputSchemaIds": ["agent.semantic-protocols.semantic-search-packet"],
        "requiresQuery": search_view_requires_query(view),
        "acceptsStdin": view == "ingest",
        "supportsPackageScope": true,
        "supportsJson": true,
        "supportsCompact": true
    });
    let Value::Object(fields) = &mut descriptor else {
        return descriptor;
    };
    let accepted_pipes = accepted_search_pipes(view);
    if !accepted_pipes.is_empty() {
        fields.insert("acceptedPipes".to_string(), json!(accepted_pipes));
    }
    let capabilities = search_capabilities(view);
    if !capabilities.is_empty() {
        fields.insert("capabilities".to_string(), json!(capabilities));
    }
    let ingest_required_for = search_ingest_required_for(view);
    if !ingest_required_for.is_empty() {
        fields.insert("ingestRequiredFor".to_string(), json!(ingest_required_for));
    }
    descriptor
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

fn accepted_search_pipes(view: &str) -> Vec<&'static str> {
    match view {
        "owner" => vec!["items"],
        "dependency" => vec!["items", "public-api", "docs", "tests"],
        "deps" => vec!["public-api"],
        "features" => vec!["cfg", "owners", "tests"],
        "ingest" => vec!["items", "tests"],
        _ => Vec::new(),
    }
}

fn search_capabilities(view: &str) -> Vec<Value> {
    match view {
        "workspace" => vec![
            semantic_capability("workspace-router"),
            rust_capability("cargo-workspace-search"),
        ],
        "prime" => vec![
            semantic_capability("package-prime-map"),
            rust_capability("rust-module-tree-prime"),
        ],
        "owner" => vec![
            semantic_capability("reasoning-owner-search"),
            rust_capability("parser-visible-module-owner-search"),
            rust_capability("test-owner-search"),
            semantic_capability("path-owner-fallback"),
        ],
        "dependency" => vec![
            semantic_capability("dependency-manifest-search"),
            rust_capability("dependency-local-usage-search"),
        ],
        "deps" => vec![
            semantic_capability("dependency-manifest-search"),
            rust_capability("dependency-local-usage-search"),
            semantic_capability("dependency-version-scope"),
            rust_capability("dependency-api-token-usage-search"),
        ],
        "features" => vec![rust_capability("cargo-feature-search")],
        "targets" => vec![rust_capability("cargo-target-search")],
        "cfg" => vec![rust_capability("cargo-cfg-search")],
        "symbol" => vec![rust_capability("symbol-definition-search")],
        "callsite" => vec![rust_capability("owner-callsite-search")],
        "import" => vec![rust_capability("import-edge-search")],
        "tests" => vec![rust_capability("test-owner-search")],
        "text" => vec![
            semantic_capability("owner-path-text-search"),
            rust_capability("parser-visible-source-text-search"),
        ],
        "patterns" | "pattern" => vec![rust_capability("pattern-recipe-search")],
        "docs" => vec![rust_capability("native-docs-api-search")],
        "docs-use" => vec![
            rust_capability("native-docs-api-search"),
            rust_capability("owner-callsite-search"),
        ],
        "api" => vec![rust_capability("native-api-shape-search")],
        "public-external-types" => vec![rust_capability("public-external-type-search")],
        "ingest" => vec![
            semantic_capability("external-candidate-ingest"),
            semantic_capability("stdin-shape-detection"),
            semantic_capability("owner-grouped-ingest"),
        ],
        _ => Vec::new(),
    }
}

fn search_ingest_required_for(view: &str) -> Vec<Value> {
    match view {
        "owner" => vec![rust_ingest_surface("non-parser-path")],
        "text" => vec![
            rust_ingest_surface("non-parser-text"),
            rust_ingest_surface("docs-text"),
            rust_ingest_surface("schema-json"),
            rust_ingest_surface("generated-artifact"),
        ],
        _ => Vec::new(),
    }
}

fn semantic_capability(name: &str) -> Value {
    capability("semantic", name)
}

fn rust_capability(name: &str) -> Value {
    capability("rust", name)
}

fn rust_ingest_surface(name: &str) -> Value {
    capability("rust", name)
}

fn capability(namespace: &str, name: &str) -> Value {
    json!({
        "languageId": "rust",
        "namespace": namespace,
        "name": name
    })
}

fn display_absolute_cli_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
