#![allow(unused_imports)]

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::cli::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin, write_manifest,
    write_search_fixture,
};

#[test]
fn cli_search_json_and_trace_follow_rfc_output_modes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let json = run_cli([
        "search".as_ref(),
        "prime".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(json.status.success(), "{json:?}");
    let stdout = String::from_utf8(json.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("search json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-search-packet"
    );
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["protocolVersion"], "1");
    assert_eq!(value["languageId"], "rust");
    assert_eq!(value["providerId"], "rs-harness");
    assert_eq!(value["binary"], "rs-harness");
    assert_eq!(
        value["namespace"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["method"], "search/prime");
    assert_eq!(value["view"], "prime");
    assert_eq!(value["renderMode"], "graph");
    assert_eq!(value["header"]["kind"], "search-prime");
    assert!(value["packages"].as_array().expect("packages").len() == 1);
    assert!(value["owners"].as_array().expect("owners").len() > 1);
    assert!(!value["edges"].as_array().expect("edges").is_empty());
    assert_eq!(value["searchSynthesis"]["algorithm"], "owner-rank-frontier");
    assert_eq!(value["searchSynthesis"]["scope"], "prime");
    assert!(
        value["searchSynthesis"]["highImpactOwners"]
            .as_array()
            .expect("high impact owners")
            .iter()
            .any(|path| path.as_str() == Some("src/lib.rs")),
        "{value}"
    );
    assert!(value.get("compact").is_none(), "{value}");

    let trace = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "items".as_ref(),
        "--trace".as_ref(),
        "--view".as_ref(),
        "both".as_ref(),
        root.as_os_str(),
    ]);
    assert!(trace.status.success(), "{trace:?}");
    let stdout = String::from_utf8(trace.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[search-trace] source=dependency query=serde pipes=items view=both"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|stage cargo=1 owners=2 items="),
        "{stdout}"
    );
    assert!(stdout.contains(" final=true lines="), "{stdout}");
    assert!(stdout.contains("[search-dependency] q=serde"), "{stdout}");
}
