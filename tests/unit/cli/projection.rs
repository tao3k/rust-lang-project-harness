use std::fs;

#[test]
fn cli_projection_emits_parser_owned_language_projection() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let root = temp.path();
    super::support::write_manifest(root, "projection-fixture");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub struct Engine;\npub fn print_query_wrapper_view() {}\n",
    )
    .expect("write source");

    let output = super::support::run_cli([
        "projection".as_ref(),
        "src/lib.rs".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
        "--json".as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let packet = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("projection");
    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.semantic-language-projection"
    );
    assert_eq!(packet["languageId"], "rust");
    assert_eq!(packet["sources"][0]["path"], "src/lib.rs");
    let items = packet["items"].as_array().expect("items");
    assert!(items.iter().any(|item| {
        item["name"] == "print_query_wrapper_view"
            && item["selector"] == "rust://src/lib.rs#item/function/print_query_wrapper_view"
    }));
}

#[test]
fn cli_projection_rejects_workspace_escape() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let output = super::support::run_cli([
        "projection".as_ref(),
        "../outside.rs".as_ref(),
        "--workspace".as_ref(),
        temp.path().as_os_str(),
        "--json".as_ref(),
    ]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("projection owner must be a relative workspace path")
    );
}
