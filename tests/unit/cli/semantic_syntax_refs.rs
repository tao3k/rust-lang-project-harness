#[path = "../../../src/cli/semantic_syntax_refs.rs"]
mod subject;

use serde_json::json;
use subject::{
    RUST_OWNER_ITEMS_QUERY_REF, attach_syntax_refs_to_source_windows,
    syntax_refs_from_read_plan_symbols,
};

#[test]
fn source_window_syntax_refs_use_item_kind_and_location() {
    let mut windows = vec![json!({
        "ownerPath": "src/lib.rs",
        "itemName": "load",
        "itemKind": "fn",
        "location": { "path": "src/lib.rs", "lineRange": "6:6" },
        "read": "src/lib.rs:6:6",
        "lineCount": 1,
        "reason": "direct-selector",
        "truncated": false
    })];

    let refs = attach_syntax_refs_to_source_windows(&mut windows).expect("syntax refs");

    assert_eq!(refs.query_ref, RUST_OWNER_ITEMS_QUERY_REF);
    assert_eq!(refs.match_refs, vec!["match.1"]);
    assert_eq!(refs.capture_refs, vec!["capture.1"]);
    assert_eq!(
        refs.anchor.expect("anchor"),
        json!({
            "nodeType": "function_item",
            "field": "name",
            "capture": "function.name",
            "location": { "path": "src/lib.rs", "lineRange": "6:6" }
        })
    );
    assert_eq!(windows[0]["fields"]["syntaxMatchRef"], "match.1");
    assert_eq!(windows[0]["fields"]["syntaxNodeType"], "function_item");
}

#[test]
fn read_plan_symbol_syntax_refs_use_symbol_read_locator() {
    let read_plan = json!({
        "symbols": [
            {
                "itemName": "load",
                "itemKind": "fn",
                "lineRange": "6:6",
                "read": "src/lib.rs:6:6"
            }
        ]
    });

    let refs = syntax_refs_from_read_plan_symbols(&read_plan).expect("syntax refs");

    assert_eq!(refs.query_ref, RUST_OWNER_ITEMS_QUERY_REF);
    assert_eq!(refs.match_refs, vec!["match.1"]);
    assert_eq!(refs.capture_refs, vec!["capture.1"]);
    assert_eq!(refs.anchor.expect("anchor")["location"]["lineRange"], "6:6");
}
