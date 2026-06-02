use serde_json::Value;
use tempfile::TempDir;

use super::support::{run_cli, run_search, write_search_fixture};

#[test]
fn cli_search_query_routes_code_shaped_use_through_native_syntax_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let query = run_search(
        root,
        &["query", "pub use Thing", "owner", "--view", "seeds"],
    );

    assert!(
        query.starts_with("[search-query] q=pub use Thing pkg=. intent=rust-use own=1 fact="),
        "{query}"
    );
    assert!(
        query.contains("|query intent=rust-use term=Thing status=hit routed=native-syntax"),
        "{query}"
    );
    assert!(
        query.contains("|fact rust:src/lib.rs:4:reexport:Thing kind=reexport source=native-parser owner=src/lib.rs line=4 visibility=public name=Thing qualifiedName=domain::Thing languageKind=use exported=true query=Thing"),
        "{query}"
    );
    assert!(query.contains("|seed owner:src/lib.rs"), "{query}");
}

#[test]
fn cli_search_fzf_points_code_shaped_queries_to_explicit_query_api() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let query = run_search(root, &["fzf", "pub use Thing", "tests", "--view", "seeds"]);

    assert!(
        query.starts_with("[search-fzf] q=pub use Thing pkg=. skipped=code-shaped-query"),
        "{query}"
    );
    assert!(
        query.contains("|query intent=code-shaped status=skipped"),
        "{query}"
    );
    assert!(query.contains("next=search-query"), "{query}");
    assert!(!query.contains("source=native-parser"), "{query}");
}

#[test]
fn cli_search_query_json_embeds_native_syntax_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "query".as_ref(),
        "pub use Thing".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("search query json");

    assert_eq!(value["method"], "search/query");
    assert_eq!(value["view"], "query");
    assert_eq!(value["query"], "pub use Thing");
    let facts = value["nativeSyntaxFacts"]
        .as_array()
        .expect("native syntax facts");
    assert!(
        facts.iter().any(|fact| {
            fact["kind"] == "reexport"
                && fact["source"] == "native-parser"
                && fact["ownerPath"] == "src/lib.rs"
                && fact["name"] == "Thing"
        }),
        "{value}"
    );
}
