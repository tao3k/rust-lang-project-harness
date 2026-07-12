use std::ffi::OsString;

use tempfile::TempDir;

use super::function_name_query_args;
use crate::cli::support::run_cli;

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

    let output = run_cli(function_name_query_args(root, &["--json"]));
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    let execution = &packet["execution"];
    assert_eq!(execution["engine"], "tree-sitter-querycursor");
    assert_eq!(
        execution["predicateEvaluator"],
        "asp-tree-sitter-predicate-v1"
    );
    assert_eq!(execution["matchStatus"], "hit");
    assert_eq!(execution["selectedFileCount"], 1);
    assert_eq!(execution["parsedFileCount"], 1);
    assert_eq!(execution["queryCompileCount"], 1);
    assert_eq!(execution["cursorMatchCount"], 1);
    assert!(
        execution["elapsedMs"]
            .as_u64()
            .is_some_and(|elapsed| elapsed < 50),
        "{execution}"
    );
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
        match_value["sourceLocation"],
        serde_json::json!({
            "path": "src/lib.rs",
            "lineRange": "1:3",
            "location": {"path": "src/lib.rs", "lineRange": "1:3"},
            "sourceLocator": "src/lib.rs:1:3",
            "sourceSpanLocator": "src/lib.rs:1:3"
        })
    );
    assert_eq!(
        match_value["nativeFactRefs"],
        serde_json::json!([native_ref])
    );
    assert_eq!(match_value["fields"]["symbol"], "exposed");
    assert_eq!(match_value["fields"]["itemRead"], "src/lib.rs:1:3");

    let capture = &match_value["captures"][0];
    assert_eq!(capture["id"], "capture.1");
    assert_eq!(capture["name"], "function.name");
    assert_eq!(capture["nodeType"], "identifier");
    assert_eq!(capture["field"], "name");
    assert_eq!(capture["range"]["lineRange"], "1:1");
    assert_eq!(
        capture["sourceLocation"],
        serde_json::json!({
            "path": "src/lib.rs",
            "lineRange": "1:1",
            "location": {"path": "src/lib.rs", "lineRange": "1:1"},
            "sourceLocator": "src/lib.rs:1:1",
            "sourceSpanLocator": "src/lib.rs:1:1"
        })
    );
    assert_eq!(capture["nativeFactRefs"], serde_json::json!([native_ref]));
    assert_eq!(capture["fields"]["nativeNodeType"], "function_item");
    assert_eq!(capture["fields"]["semanticKind"], "function");
    assert_eq!(capture["fields"]["sourceAuthority"], "native-parser");
    assert_eq!(capture["fields"]["read"], "src/lib.rs:1:1");
    assert_eq!(packet["cache"]["rawSourceStored"], false);
}

#[test]
fn tree_sitter_query_compact_output_hides_structural_capture_text() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "struct Config { value: usize }\n\npub fn build() {\n    let _ = Config { value: 1 };\n}\n",
    )
    .expect("fixture");

    let output = run_cli(vec![
        OsString::from("query"),
        OsString::from("--treesitter-query"),
        OsString::from("((struct_expression) @expression)"),
        OsString::from("--asp-syntax-query-captures"),
        OsString::from("expression"),
        OsString::from("--asp-syntax-query-node-types"),
        OsString::from("struct_expression"),
        OsString::from("--workspace"),
        root.as_os_str().to_os_string(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("capture:expression(<struct_expression>)"),
        "{stdout}"
    );
    assert!(!stdout.contains("Config { value: 1 }"), "{stdout}");
}

#[test]
fn tree_sitter_query_accepts_grammar_nodes_without_a_static_whitelist() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "struct Config { value: usize }\n\npub fn build() {\n    let _ = Config { value: 1 };\n}\n",
    )
    .expect("fixture");

    let output = run_cli(vec![
        OsString::from("query"),
        OsString::from("--treesitter-query"),
        OsString::from("((struct_expression) @expression)"),
        OsString::from("--asp-syntax-query-captures"),
        OsString::from("expression"),
        OsString::from("--asp-syntax-query-node-types"),
        OsString::from("struct_expression"),
        OsString::from("--json"),
        OsString::from("--workspace"),
        root.as_os_str().to_os_string(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("semantic tree-sitter query packet JSON");
    assert_eq!(packet["execution"]["engine"], "tree-sitter-querycursor");
    assert_eq!(packet["execution"]["matchStatus"], "hit");
    assert_eq!(
        packet["execution"]["unsupportedPredicates"],
        serde_json::json!([])
    );
    assert_eq!(packet["matches"].as_array().expect("matches").len(), 1);
    assert_eq!(
        packet["matches"][0]["captures"][0]["nodeType"],
        "struct_expression"
    );
}
