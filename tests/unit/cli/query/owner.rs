use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli, write_search_fixture};

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
        stdout.starts_with("[search-owner] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|query itemQuery=load status=hit match=exact item=1 reason=parser-item-exact next=code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|item load kind=fn"), "{stdout}");
    assert!(stdout.contains("read=src/lib.rs:"), "{stdout}");
    assert!(stdout.contains("|code path=src/lib.rs"), "{stdout}");
    let query_code = run_cli([
        "query".as_ref(),
        "src/lib.rs".as_ref(),
        "--query".as_ref(),
        "load".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(query_code.status.success(), "{query_code:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(query_code.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.contains("fn load"), "{stdout}");
    assert!(stdout.contains("call domain::make_thing"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
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
    assert!(stdout.contains("call domain::make_thing"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
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
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback output=names next=code"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("|item load kind=fn"), "{stdout}");
    assert!(!stdout.contains("|code path=src/lib.rs"), "{stdout}");
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
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback next=code"
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
    assert!(
        value["matches"][0]["code"]
            .as_str()
            .expect("compact code")
            .contains("pub fn load() -> Thing"),
        "{value}"
    );
    assert_eq!(value["matches"][0]["projection"]["mode"], "compact");
    assert_eq!(
        value["matches"][0]["projection"]["syntax"],
        "semantic-outline"
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
    let nodes = value["matches"][0]["projection"]["nodes"]
        .as_array()
        .expect("projection nodes");
    assert!(
        nodes.iter().any(|node| {
            node["role"] == "declaration"
                && node["kind"] == "fn"
                && node["label"]
                    .as_str()
                    .is_some_and(|label| label.contains("fn load"))
        }),
        "{value}"
    );
    assert!(
        nodes.iter().any(|node| {
            node["role"] == "call"
                && node["kind"] == "call"
                && node["label"] == "call domain::make_thing"
        }),
        "{value}"
    );
    assert!(
        nodes.iter().all(|node| node["read"]
            .as_str()
            .is_some_and(|read| read.starts_with("src/lib.rs:"))),
        "{value}"
    );
}
