use tempfile::TempDir;

use super::{FUNCTION_NAME_QUERY, function_name_query_args};
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
            .starts_with("syntax-catalog:")
    );
    assert!(
        packet["cache"]["grammarProfileFingerprint"]
            .as_str()
            .expect("grammar profile fingerprint")
            .starts_with("grammar-profile:")
    );
}

#[test]
fn calls_catalog_packet_projects_native_call_captures() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "fn parse_query() {}\n\nstruct Runner;\nimpl Runner {\n    fn run(&self) {\n        parse_query();\n    }\n}\n",
    )
    .expect("fixture");

    let output = run_cli([
        "query".as_ref(),
        "--catalog".as_ref(),
        "calls".as_ref(),
        "--term".as_ref(),
        "parse_query".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(packet["matches"].as_array().expect("matches").len(), 1);
    let capture = &packet["matches"][0]["captures"][0];
    assert_eq!(capture["name"], "call.target");
    assert_eq!(capture["nodeType"], "call_expression");
    assert_eq!(capture["field"], "function");
    assert_eq!(capture["fields"]["semanticKind"], "call");
    assert_eq!(capture["fields"]["read"], "src/lib.rs:6:6");
    assert_eq!(capture["fields"]["itemRead"], "src/lib.rs:5:7");
    assert!(
        capture["nativeFactRefs"][0]
            .as_str()
            .expect("native fact ref")
            .starts_with("rust:syntax:src/lib.rs:5:7:parse_query")
    );
}

#[test]
fn tree_sitter_query_packet_accepts_inline_s_expression() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();

    let output = run_cli(function_name_query_args(root, &["--json"]));
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.semantic-tree-sitter-query"
    );
    assert_eq!(packet["grammarId"], "tree-sitter-rust");
    assert_eq!(packet["query"]["inputForm"], "s-expression");
    assert_eq!(packet["query"]["input"], FUNCTION_NAME_QUERY);
    assert_eq!(packet["query"]["compiledSource"], FUNCTION_NAME_QUERY);
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
fn direct_provider_inline_query_requires_asp_compiled_plan() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();

    let output = run_cli([
        "query".as_ref(),
        "--treesitter-query".as_ref(),
        FUNCTION_NAME_QUERY.as_ref(),
        root.as_os_str(),
    ]);
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("requires ASP-compiled query plan"),
        "{stderr}"
    );
}
