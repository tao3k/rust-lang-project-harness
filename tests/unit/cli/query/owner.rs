use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_manifest, write_search_fixture};

#[test]
fn cli_help_advertises_code_flag() {
    let top = run_cli(["--help"]);
    assert!(top.status.success(), "{top:?}");
    let stdout = String::from_utf8(top.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("rs-harness search <view> [ARGS] [PIPE...] [--json] [--code]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("rs-harness query [SELECTOR] [--query SYMBOL | --term TERM] [--code]"),
        "{stdout}"
    );

    let search = run_cli(["search", "--help"]);
    assert!(search.status.success(), "{search:?}");
    let stdout = String::from_utf8(search.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("items --query SYMBOL [--names-only | --code]"),
        "{stdout}"
    );
    assert!(stdout.contains("--names-only, --code, --lines"), "{stdout}");

    let query = run_cli(["query", "--help"]);
    assert!(query.status.success(), "{query:?}");
    let stdout = String::from_utf8(query.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[--query SYMBOL] [--names-only | --code]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Use --code after selecting an owner/symbol or hook path/range"),
        "{stdout}"
    );
}

#[test]
fn cli_query_owner_selector_extracts_parser_item_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with("[query-item] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact next=query-code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|item load kind=fn"), "{stdout}");
    assert!(stdout.contains("read=src/lib.rs:"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let exact_names_only = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--names-only".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(exact_names_only.status.success(), "{exact_names_only:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(exact_names_only.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.starts_with(
            "[query-item] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load output=names"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact output=names next=query-code"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("|item load kind=fn next=syntax:load read=src/lib.rs:"),
        "{stdout}"
    );
    assert!(stdout.contains("syn=function_item/name"), "{stdout}");
    assert!(
        !stdout.contains("responsibilities="),
        "exact names-only query should stay on the local locator fast path: {stdout}"
    );
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let query_code = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(query_code.status.success(), "{query_code:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(query_code.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.contains("fn load"), "{stdout}");
    assert!(stdout.contains("domain::make_thing()"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[query-item]"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let search_code = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(search_code.status.success(), "{search_code:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(search_code.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.contains("fn load"), "{stdout}");
    assert!(stdout.contains("domain::make_thing()"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[query-item]"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let names_only = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "loa".as_ref(),
        "--names-only".as_ref(),
        root.as_os_str(),
    ]);
    assert!(names_only.status.success(), "{names_only:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(names_only.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.contains(
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback output=names next=query-code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|item load kind=fn"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
}

#[test]
fn cli_query_owner_code_preserves_original_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-code-raw");
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    let source = "pub fn keep_spacing( input : String )->String{\n    input.clone()\n}\n";
    std::fs::write(root.join("src/lib.rs"), source).expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "keep_spacing".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, source);
    assert!(!stdout.contains("keep_spacing(input: String) -> String"));
}

#[test]
fn cli_query_selector_range_code_uses_local_window() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-selector-range-code");
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    let source = "pub fn selected() {\n    println!(\"selected\");\n}\n\npub fn skipped() {}\n";
    std::fs::write(root.join("src/lib.rs"), source).expect("write source");

    let output = run_cli([
        "query".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:2".as_ref(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "pub fn selected() {\n    println!(\"selected\");\n");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("skipped"), "{stdout}");
}

#[test]
fn cli_query_owner_selector_reports_fallback_and_miss_accuracy() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let fallback = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "loa".as_ref(),
        root.as_os_str(),
    ]);
    assert!(fallback.status.success(), "{fallback:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(fallback.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.contains(
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback next=query-code"
        ),
        "{stdout}"
    );
    let miss = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "DefinitelyMissingSymbol".as_ref(),
        root.as_os_str(),
    ]);
    assert!(miss.status.success(), "{miss:?}");
    let stdout = normalize_temp_root(&String::from_utf8(miss.stdout).expect("utf8 stdout"), root);
    assert!(
        stdout.contains(
            "|query itemQuery=DefinitelyMissingSymbol status=miss match=none item=0 reason=parser-item-miss next=revise-query"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let candidate = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--term".as_ref(),
        "load_missing".as_ref(),
        "--names-only".as_ref(),
        root.as_os_str(),
    ]);
    assert!(candidate.status.success(), "{candidate:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(candidate.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load_missing status=miss match=none item=0 reason=parser-item-miss output=names candidates=load next=query:load"
        ),
        "{stdout}"
    );
}

#[test]
fn cli_query_owner_selector_json_uses_query_packet() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--term".as_ref(),
        "load_missing".as_ref(),
        "--names-only".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("query json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-query-packet"
    );
    assert_eq!(value["method"], "query/owner-items");
    assert_eq!(value["ownerPath"], "src/lib.rs");
    assert_eq!(value["outputMode"], "names");
    assert_eq!(value["queryCoverage"][0]["value"], "load_missing");
    assert_eq!(value["queryCoverage"][0]["status"], "miss");
    assert_eq!(value["queryCoverage"][0]["candidateNames"][0], "load");
    assert_eq!(value["candidateItems"][0]["name"], "load");
    assert!(
        value["matches"].as_array().expect("matches").is_empty(),
        "{value}"
    );

    let exact = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--names-only".as_ref(),
        "--json".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(exact.status.success(), "{exact:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&exact.stdout).expect("query json");
    assert_eq!(value["method"], "query/owner-items");
    assert_eq!(value["ownerPath"], "src/lib.rs");
    assert_eq!(value["outputMode"], "names");
    assert_eq!(value["queryCoverage"][0]["value"], "load");
    assert_eq!(value["queryCoverage"][0]["status"], "hit");
    assert_eq!(value["queryCoverage"][0]["match"], "exact");
    assert_eq!(value["matches"][0]["name"], "load");
    assert_eq!(value["matches"][0]["kind"], "fn");
    assert_eq!(value["matches"][0]["code"], serde_json::Value::Null);
    assert_eq!(
        value["matches"][0]["fields"]["syntaxNodeType"],
        "function_item"
    );
    assert_eq!(
        value["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
}

#[test]
fn cli_query_owner_selector_json_marks_compact_projection() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("query json");
    assert_eq!(value["outputMode"], "code");
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
    assert!(value["matches"][0]["code"].is_null(), "{value}");
    assert_eq!(
        value["matches"][0]["fields"]["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
    assert_eq!(value["matches"][0]["fields"]["syntaxMatchRef"], "match.1");
    assert_eq!(
        value["matches"][0]["fields"]["syntaxCaptureRef"],
        "capture.1"
    );
    assert_eq!(
        value["matches"][0]["fields"]["syntaxNodeType"],
        "function_item"
    );
    assert_eq!(
        value["matches"][0]["fields"]["syntaxCapture"],
        "function.name"
    );
    assert_eq!(value["matches"][0]["projection"]["mode"], "compact");
    assert_eq!(
        value["matches"][0]["projection"]["syntax"],
        "save-token-rustfmt"
    );
    assert_eq!(
        value["matches"][0]["projection"]["sourceAuthority"],
        "native-parser"
    );
    assert_eq!(value["matches"][0]["projection"]["losslessStructure"], true);
    assert!(
        value["matches"][0]["projection"]["exactRead"]
            .as_str()
            .expect("exact read")
            .starts_with("src/lib.rs:"),
        "{value}"
    );
}
