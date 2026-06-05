use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn query_catalog_packet_uses_binary_embedded_sources() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    assert!(!root.join("tree-sitter").exists());

    let output = run_cli([
        "query".as_ref(),
        "--catalog".as_ref(),
        "calls".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.semantic-tree-sitter-query"
    );
    assert_eq!(packet["grammarId"], "tree-sitter-rust");
    assert_eq!(packet["query"]["catalogId"], "calls");
    assert_eq!(
        packet["query"]["catalogPath"],
        "tree-sitter/tree-sitter-rust/queries/calls.scm"
    );
    assert_eq!(
        packet["query"]["grammarProfilePath"],
        "tree-sitter/tree-sitter-rust/grammar-profile.json"
    );
    assert!(
        packet["query"]["compiledSource"]
            .as_str()
            .expect("compiled source")
            .contains("call_expression")
    );
    assert_eq!(
        packet["query"]["fields"]["captures"],
        serde_json::json!(["call.expression", "call.method", "call.target"])
    );
    assert_eq!(packet["query"]["fields"]["catalogEmbedded"], true);
    assert!(
        packet["cache"]["catalogFingerprint"]
            .as_str()
            .expect("catalog fingerprint")
            .starts_with("rust-default:")
    );
    assert!(
        packet["cache"]["grammarProfileFingerprint"]
            .as_str()
            .expect("grammar profile fingerprint")
            .starts_with("rust-default:")
    );
}

#[test]
fn tree_sitter_query_packet_accepts_inline_s_expression() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let query = "(function_item name: (identifier) @function.name)";

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        query.as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.semantic-tree-sitter-query"
    );
    assert_eq!(packet["grammarId"], "tree-sitter-rust");
    assert_eq!(packet["query"]["inputForm"], "s-expression");
    assert_eq!(packet["query"]["input"], query);
    assert_eq!(packet["query"]["compiledSource"], query);
    assert!(packet["query"].get("catalogId").is_none());
    assert!(packet["query"].get("catalogPath").is_none());
    assert_eq!(
        packet["query"]["fields"]["captures"],
        serde_json::json!(["function.name"])
    );
    assert_eq!(packet["query"]["fields"]["catalogCanonical"], false);
    assert_eq!(packet["query"]["fields"]["catalogEmbedded"], false);
}

#[test]
fn tree_sitter_query_locator_output_names_captures_without_artifact_scope() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn exposed() -> usize {\n    1\n}\n\nstruct Hidden;\n",
    )
    .expect("fixture");
    let query = "(function_item name: (identifier) @function.name)";

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        query.as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "src/lib.rs:1\nexposed\n");
    assert!(!stdout.contains("pub fn exposed()"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
    assert!(!stdout.contains("clientDbHint="));
    assert!(!stdout.contains("matches=0"));
}

#[test]
fn tree_sitter_query_json_projects_matches_and_native_enrichment() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn exposed() -> usize {\n    1\n}\n\nstruct Hidden;\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        "(function_item name: (identifier) @function.name)".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(packet["matches"].as_array().expect("matches").len(), 1);
    assert!(packet["query"]["fields"].get("selector").is_none());
    let native_ref = packet["nativeFactRefs"][0]
        .as_str()
        .expect("native fact ref");
    assert_eq!(native_ref, "rust:item:src/lib.rs:1:3:exposed");

    let match_value = &packet["matches"][0];
    assert_eq!(match_value["id"], "match.1");
    assert_eq!(match_value["range"]["path"], "src/lib.rs");
    assert_eq!(match_value["range"]["lineRange"], "1:3");
    assert_eq!(
        match_value["nativeFactRefs"],
        serde_json::json!([native_ref])
    );
    assert_eq!(match_value["fields"]["symbol"], "exposed");
    assert_eq!(match_value["fields"]["itemRead"], "src/lib.rs:1:3");

    let capture = &match_value["captures"][0];
    assert_eq!(capture["id"], "capture.1");
    assert_eq!(capture["name"], "function.name");
    assert_eq!(capture["nodeType"], "function_item");
    assert_eq!(capture["field"], "name");
    assert_eq!(capture["range"]["lineRange"], "1:1");
    assert_eq!(capture["nativeFactRefs"], serde_json::json!([native_ref]));
    assert_eq!(capture["fields"]["semanticKind"], "function");
    assert_eq!(capture["fields"]["sourceAuthority"], "native-parser");
    assert_eq!(capture["fields"]["read"], "src/lib.rs:1:1");
    assert_eq!(packet["cache"]["rawSourceStored"], false);
}

#[test]
fn tree_sitter_query_locator_output_can_be_filtered_by_term() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        "(function_item name: (identifier) @function.name)".as_ref(),
        "--term".as_ref(),
        "beta".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "src/lib.rs:5\nbeta_target\n");
    assert!(!stdout.contains("name=alpha"));
    assert!(!stdout.contains("pub fn"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
}

#[test]
fn tree_sitter_query_exact_selector_code_output_is_plain_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn locate_me() -> usize {\n    7\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        "(function_item name: (identifier) @function.name)".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:3".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "pub fn locate_me() -> usize {\n    7\n}\n");
}

#[test]
fn tree_sitter_query_single_line_locator_can_drive_exact_code_output() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        "(function_item name: (identifier) @function.name)".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:5".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "pub fn beta_target() -> usize {\n    2\n}\n");
}

#[test]
fn rust_tree_sitter_queries_follow_upstream_layout() {
    let provider_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tree-sitter")
        .join("tree-sitter-rust");
    assert!(provider_root.join("corpus-profile.json").is_file());
    assert!(
        std::fs::read_dir(&provider_root)
            .expect("provider tree-sitter-rust dir")
            .filter_map(Result::ok)
            .all(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("scm")),
        "Rust tree-sitter queries must live under queries/"
    );

    let queries_root = provider_root.join("queries");
    for name in [
        "calls.scm",
        "cfg.scm",
        "declarations.scm",
        "imports.scm",
        "injections.scm",
        "macros.scm",
        "tags.scm",
    ] {
        assert!(queries_root.join(name).is_file(), "missing {name}");
    }
}
