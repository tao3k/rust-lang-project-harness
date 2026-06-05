use std::fs;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};

#[test]
fn broad_fallback_item_query_auto_uses_names_only() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    fs::write(
        root.join("src/lib.rs"),
        r#"
pub fn load() {}
pub fn save() {}
pub fn domain() {}
pub fn make() {}
pub fn thing() {}
"#,
    )
    .expect("write broad query fixture");

    let output = run_cli([
        "search",
        "owner",
        "src/lib.rs",
        "items",
        "--query",
        "loa|sav|dom|mak|thi",
        root.to_str().expect("root path"),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = normalize_temp_root(&stdout_raw, root);

    assert!(stdout.contains("match=fallback-contains"), "{stdout}");
    assert!(stdout.contains("output=names"), "{stdout}");
    assert!(stdout.contains("|item load kind=fn"), "{stdout}");
    assert!(stdout.contains("|item thing kind=fn"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    assert!(!stdout.contains(" text=\""), "{stdout}");
}

#[test]
fn broad_fallback_item_query_reports_term_revisions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    fs::write(
        root.join("src/lib.rs"),
        r#"
pub fn render_query_json() {}
pub fn projection_nodes_from_compact_code() {}
pub fn projection_nodes_from_parser_fields() {}
pub fn projection_node_from_parser_token() {}
pub fn projection_node_classification() {}
pub fn projection_expand_actions() {}
"#,
    )
    .expect("write stale query fixture");

    let output = run_cli([
        "search",
        "owner",
        "src/lib.rs",
        "items",
        "--query",
        "render_semantic_query_json|projection_from_code_line|projection_node|expandActions",
        root.to_str().expect("root path"),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout_raw = String::from_utf8_lossy(&output.stdout);
    let stdout = normalize_temp_root(&stdout_raw, root);

    assert!(stdout.contains("output=names"), "{stdout}");
    assert!(stdout.contains("next=select-item"), "{stdout}");
    assert!(
        stdout.contains("revise=render_semantic_query_json->render_query_json"),
        "{stdout}"
    );
    assert!(
        stdout.contains("projection_from_code_line->projection_nodes_from_compact_code"),
        "{stdout}"
    );
    assert!(
        stdout.contains("expandActions->projection_expand_actions"),
        "{stdout}"
    );
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
}
