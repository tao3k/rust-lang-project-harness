use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_behavior_snapshot_renders_json_contract() {
    let output = run_cli([
        "behavior",
        "snapshot",
        "--kind",
        "public-api",
        "--path",
        "src/lib.rs",
        "--symbol",
        "parse_public_api_shape",
        "--expected",
        "pub fn parse_public_api_shape(...)",
        "--actual",
        "pub fn parse_public_api_shape(...)",
        "--receipt-id",
        "rust.expect-test:expect-test:passed",
        "--candidate-id",
        "agent-r027:src/lib.rs:1",
        "--json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("behavior json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-behavior-snapshot"
    );
    assert_eq!(value["subject"]["kind"], "public-api");
    assert_eq!(value["subject"]["path"], "src/lib.rs");
    assert_eq!(value["status"], "matched");
    assert_eq!(value["observations"][0]["kind"], "snapshot");
    assert_eq!(
        value["receiptIds"],
        serde_json::json!(["rust.expect-test:expect-test:passed"])
    );
    assert_eq!(
        value["candidateIds"],
        serde_json::json!(["agent-r027:src/lib.rs:1"])
    );
}

#[test]
fn cli_behavior_snapshot_infers_changed_when_actual_differs() {
    let output = run_cli([
        "behavior",
        "snapshot",
        "--kind",
        "cli",
        "--path",
        "src/cli/runner.rs",
        "--expected",
        "old",
        "--actual",
        "new",
        "--json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("behavior json");
    assert_eq!(value["status"], "changed");
    assert_eq!(
        value["observations"][0]["message"],
        "behavior snapshot changed"
    );
}

#[test]
fn cli_behavior_snapshot_links_expect_test_receipt_json() {
    let temp = TempDir::new().expect("temp dir");
    let receipt_path = temp.path().join("expect-receipt.json");
    fs::write(
        &receipt_path,
        serde_json::json!({
            "schemaId": "agent.semantic-protocols.semantic-verification-receipt",
            "schemaVersion": "1",
            "protocolId": "agent.semantic-protocols.verification-receipt",
            "protocolVersion": "1",
            "receiptId": "rust.expect-test:expect-test:failed",
            "producer": {
                "languageId": "rust",
                "providerId": "rs-harness",
                "adapterId": "rust.expect-test",
                "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
            },
            "tool": "expect-test",
            "status": "failed",
            "command": {
                "argv": ["cargo", "test"],
                "outputFormat": "expect-test"
            },
            "exitCode": 101,
            "summary": "expect-test snapshot changed",
            "observations": [
                {
                    "kind": "snapshot-diff",
                    "message": "snapshot output differed"
                }
            ]
        })
        .to_string(),
    )
    .expect("write receipt");
    let output = run_cli([
        "behavior".as_ref(),
        "snapshot".as_ref(),
        "--kind".as_ref(),
        "public-api".as_ref(),
        "--path".as_ref(),
        "src/lib.rs".as_ref(),
        "--receipt-json".as_ref(),
        receipt_path.as_os_str(),
        "--json".as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("behavior json");
    assert_eq!(value["status"], "changed");
    assert_eq!(
        value["receiptIds"],
        serde_json::json!(["rust.expect-test:expect-test:failed"])
    );
    assert_eq!(value["observations"][1]["kind"], "snapshot");
    assert_eq!(value["observations"][1]["fields"]["tool"], "expect-test");
}
