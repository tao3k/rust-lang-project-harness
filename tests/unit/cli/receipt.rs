use serde_json::Value;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_receipt_dry_run_renders_json_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let output = run_cli([
        "receipt".as_ref(),
        "cargo-check".as_ref(),
        "--dry-run".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("receipt json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-verification-receipt"
    );
    assert_eq!(value["tool"], "cargo-check");
    assert_eq!(value["status"], "skipped");
    assert_eq!(
        value["command"]["argv"],
        serde_json::json!(["cargo", "check", "--message-format=json"])
    );
    assert_eq!(value["observations"][0]["kind"], "note");
}

#[test]
fn cli_receipt_proptest_case_filter_shapes_adapter_command() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let output = run_cli([
        "receipt".as_ref(),
        "proptest".as_ref(),
        "--case-filter".as_ref(),
        "prop_roundtrip".as_ref(),
        "--dry-run".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("receipt json");
    assert_eq!(value["tool"], "proptest");
    assert_eq!(value["command"]["outputFormat"], "libtest");
    assert_eq!(
        value["command"]["argv"],
        serde_json::json!(["cargo", "test", "prop_roundtrip", "--", "--nocapture"])
    );
}

#[test]
fn cli_receipt_cargo_fuzz_target_shapes_adapter_command() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let output = run_cli([
        "receipt".as_ref(),
        "cargo-fuzz".as_ref(),
        "--target".as_ref(),
        "parser".as_ref(),
        "--runs".as_ref(),
        "8".as_ref(),
        "--dry-run".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("receipt json");
    assert_eq!(value["tool"], "cargo-fuzz");
    assert_eq!(value["command"]["outputFormat"], "fuzz-corpus");
    assert_eq!(
        value["command"]["argv"],
        serde_json::json!(["cargo", "fuzz", "run", "parser", "--", "-runs=8"])
    );
}

#[test]
fn cli_receipt_kani_harness_shapes_adapter_command() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let output = run_cli([
        "receipt".as_ref(),
        "kani".as_ref(),
        "--harness".as_ref(),
        "parser_facts".as_ref(),
        "--dry-run".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("receipt json");
    assert_eq!(value["tool"], "kani");
    assert_eq!(value["command"]["outputFormat"], "proof-report");
    assert_eq!(
        value["command"]["argv"],
        serde_json::json!(["cargo", "kani", "--harness", "parser_facts"])
    );
}

#[test]
fn cli_receipt_verus_file_shapes_adapter_command() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let output = run_cli([
        "receipt".as_ref(),
        "verus".as_ref(),
        "--file".as_ref(),
        "proofs/parser_facts.rs".as_ref(),
        "--dry-run".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("receipt json");
    assert_eq!(value["tool"], "verus");
    assert_eq!(value["command"]["outputFormat"], "proof-report");
    assert_eq!(
        value["command"]["argv"],
        serde_json::json!(["verus", "proofs/parser_facts.rs"])
    );
}
