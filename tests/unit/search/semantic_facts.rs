use std::fs;

use rust_lang_project_harness::render_rust_project_harness_search_semantic_facts_json;
use serde_json::Value as JsonValue;

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
    fs::write(tempdir.path().join("src/lexical.rs"), "fn vec_hit() {}\n").expect("write lexical");

    let rendered = render_rust_project_harness_search_semantic_facts_json(
        tempdir.path(),
        "Vec scalar collection fields",
        "src/lexical.rs:1:1:Vec\n",
    )
    .expect("render facts");
    let packet: JsonValue = serde_json::from_str(&rendered).expect("json");
    assert_eq!(
        packet["schemaId"].as_str(),
        Some("agent.semantic-protocols.semantic-fact-graph")
    );
    assert_eq!(packet["languageId"].as_str(), Some("rust"));
    assert_eq!(packet["providerId"].as_str(), Some("rs-harness"));
    assert_eq!(
        packet["query"].as_str(),
        Some("Vec scalar collection fields")
    );
    let nodes = packet["nodes"].as_array().expect("nodes");
    let edges = packet["edges"].as_array().expect("edges");

    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("owner") && node["value"].as_str() == Some("src/model.rs")
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("field")
            && node["symbol"].as_str() == Some("scalars")
            && node["fields"]["typeValue"].as_str() == Some("Vec < Scalar >")
            && node["fields"]["languageId"].as_str() == Some("rust")
            && node["fields"]["providerId"].as_str() == Some("rs-harness")
            && node["fields"]["semanticFactKind"].as_str() == Some("field")
            && node["fields"]["provenance"].as_str() == Some("parser")
            && node["fields"]["confidence"].as_str() == Some("exact")
            && node["fields"]["freshness"].as_str() == Some("fresh")
            && node["fields"]["collectionFamily"].as_str() == Some("sequence")
            && node["fields"]["collectionImpl"].as_str() == Some("Vec")
            && node["fields"]["elementShape"].as_str() == Some("scalar")
            && node["fields"]["contextLocator"]
                .as_str()
                .is_some_and(|selector| selector.starts_with("src/model.rs:"))
            && node["fields"]["field"]["ownerKind"].as_str() == Some("struct")
            && node["fields"]["field"]["name"].as_str() == Some("scalars")
            && node["fields"]["field"]["ownerPath"].as_str() == Some("src/model.rs")
            && node["fields"]["field"]["access"]
                .as_array()
                .is_some_and(|access| access.iter().any(|mode| mode.as_str() == Some("append")))
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("type")
            && node["fields"]["fieldName"].as_str() == Some("scalars")
            && node["fields"]["semanticFactKind"].as_str() == Some("type")
            && node["fields"]["type"]["name"].as_str() == Some("Vec < Scalar >")
            && node["fields"]["type"]["element"].as_str() == Some("Scalar")
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("collection")
            && node["fields"]["semanticFactKind"].as_str() == Some("collection")
            && node["fields"]["collection"]["family"].as_str() == Some("sequence")
            && node["fields"]["collection"]["impl"].as_str() == Some("Vec")
            && node["fields"]["collection"]["elementType"].as_str() == Some("Scalar")
    }));
    assert!(
        edges
            .iter()
            .any(|edge| edge["relation"].as_str() == Some("has_type"))
    );
}

#[test]
fn semantic_facts_emit_cargo_package_build_dependency_and_test_targets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tempdir.path().join("src")).expect("create src");
    fs::create_dir_all(tempdir.path().join("tests")).expect("create tests");
    fs::write(
        tempdir.path().join("Cargo.toml"),
        "[package]\n\
         name = \"fact-crate\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [dependencies]\n\
         serde = { version = \"1\", features = [\"derive\"] }\n\
         \n\
         [dev-dependencies]\n\
         tokio = { version = \"1\", features = [\"rt\"] }\n\
         \n\
         [[test]]\n\
         name = \"api_contract\"\n\
         path = \"tests/api_contract.rs\"\n",
    )
    .expect("write manifest");
    fs::write(
        tempdir.path().join("src/lib.rs"),
        "pub struct Cache {\n    pub entries: Vec<String>,\n}\n\npub fn api() {}\n",
    )
    .expect("write lib");
    fs::write(
        tempdir.path().join("tests/api_contract.rs"),
        "#[test]\nfn api_is_callable() { fact_crate::api(); }\n",
    )
    .expect("write test");

    let rendered = render_rust_project_harness_search_semantic_facts_json(
        tempdir.path(),
        "Vec field cargo test tokio dependency",
        "src/lib.rs:2:1:entries\n",
    )
    .expect("render facts");
    let packet: JsonValue = serde_json::from_str(&rendered).expect("json");
    let nodes = packet["nodes"].as_array().expect("nodes");
    let edges = packet["edges"].as_array().expect("edges");

    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("package")
            && node["value"].as_str() == Some("fact-crate")
            && node["action"].as_str() == Some("package")
            && node["fields"]["semanticFactKind"].as_str() == Some("package")
            && node["fields"]["manifestPath"].as_str() == Some("Cargo.toml")
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("build")
            && node["action"].as_str() == Some("build")
            && node["fields"]["semanticFactKind"].as_str() == Some("build")
            && node["fields"]["command"].as_str() == Some("cargo test -p fact-crate")
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("dependency")
            && node["value"].as_str() == Some("tokio")
            && node["action"].as_str() == Some("deps")
            && node["fields"]["semanticFactKind"].as_str() == Some("dependency")
            && node["fields"]["dependencyKind"].as_str() == Some("dev")
            && node["fields"]["features"]
                .as_array()
                .is_some_and(|features| features.iter().any(|feature| feature == "rt"))
    }));
    assert!(nodes.iter().any(|node| {
        node["kind"].as_str() == Some("test")
            && node["path"].as_str() == Some("tests/api_contract.rs")
            && node["action"].as_str() == Some("tests")
            && node["fields"]["semanticFactKind"].as_str() == Some("test")
            && node["fields"]["functionCount"].as_u64() == Some(1)
            && node["fields"]["command"].as_str() == Some("cargo test -p fact-crate")
    }));
    let field_id = nodes
        .iter()
        .find(|node| {
            node["kind"].as_str() == Some("field") && node["symbol"].as_str() == Some("entries")
        })
        .and_then(|node| node["id"].as_str())
        .expect("field id");
    let package_id = nodes
        .iter()
        .find(|node| node["kind"].as_str() == Some("package"))
        .and_then(|node| node["id"].as_str())
        .expect("package id");
    assert!(edges.iter().any(|edge| {
        edge["source"].as_str() == Some(field_id)
            && edge["target"].as_str() == Some(package_id)
            && edge["relation"].as_str() == Some("belongs_to")
    }));
    for relation in ["builds", "depends_on", "tests", "belongs_to"] {
        assert!(
            edges
                .iter()
                .any(|edge| edge["relation"].as_str() == Some(relation)),
            "missing relation {relation}"
        );
    }
}
