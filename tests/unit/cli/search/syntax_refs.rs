use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_search_owner_items_json_exposes_tree_sitter_syntax_refs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("search json");

    assert_eq!(value["method"], "search/owner");
    assert_eq!(
        value["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
    assert_eq!(value["syntaxMatchRefs"], serde_json::json!(["match.1"]));
    assert_eq!(value["syntaxCaptureRefs"], serde_json::json!(["capture.1"]));
    assert_eq!(value["syntaxAnchor"]["nodeType"], "function_item");
    assert_eq!(value["syntaxAnchor"]["field"], "name");
    assert_eq!(value["syntaxAnchor"]["capture"], "function.name");
    assert_eq!(value["syntaxAnchor"]["location"]["path"], "src/lib.rs");
    assert_eq!(value["syntaxAnchor"]["location"]["lineRange"], "6:6");

    let item = &value["items"][0];
    assert_eq!(item["name"], "load");
    assert!(item.get("code").is_none(), "{value}");
    assert_eq!(
        item["fields"]["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
    assert_eq!(item["fields"]["syntaxMatchRef"], "match.1");
    assert_eq!(item["fields"]["syntaxCaptureRef"], "capture.1");
    assert_eq!(item["fields"]["syntaxNodeType"], "function_item");
    assert_eq!(item["fields"]["syntaxCapture"], "function.name");
    assert_eq!(item["fields"]["read"], "src/lib.rs:6:6");
}
