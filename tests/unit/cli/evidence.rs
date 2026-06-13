use serde_json::Value;
use std::fs;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_evidence_graph_renders_json_contract() {
    let temp = TempDir::new().expect("temp dir");
    let review_packet_path = temp.path().join("review-packet.json");
    fs::write(
        &review_packet_path,
        serde_json::to_string(&review_packet_json()).expect("review packet json"),
    )
    .expect("write review packet");

    let output = run_cli([
        "evidence".as_ref(),
        "graph".as_ref(),
        "--review-packet-json".as_ref(),
        review_packet_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("evidence graph json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-evidence-graph"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.evidence-graph"
    );
    assert_eq!(value["summary"]["owners"], 1);
    assert_eq!(value["summary"]["gaps"], 1);
    assert!(
        value["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .any(|node| node["kind"] == "review-packet"),
        "{value}"
    );
}

#[test]
fn cli_evidence_graph_rejects_wrong_review_packet_schema_id() {
    let temp = TempDir::new().expect("temp dir");
    let review_packet_path = temp.path().join("bad-review-packet.json");
    let mut packet = review_packet_json();
    packet["schemaId"] = serde_json::json!("agent.semantic-protocols.semantic-review-packet.bad");
    fs::write(
        &review_packet_path,
        serde_json::to_string(&packet).expect("review packet json"),
    )
    .expect("write review packet");

    let output = run_cli([
        "evidence".as_ref(),
        "graph".as_ref(),
        "--review-packet-json".as_ref(),
        review_packet_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("has schemaId agent.semantic-protocols.semantic-review-packet.bad"),
        "{stderr}"
    );
    assert!(
        stderr.contains("expected agent.semantic-protocols.semantic-review-packet"),
        "{stderr}"
    );
}

#[test]
fn cli_evidence_assurance_renders_json_contract() {
    let temp = TempDir::new().expect("temp dir");
    let review_packet_path = temp.path().join("review-packet.json");
    fs::write(
        &review_packet_path,
        serde_json::to_string(&review_packet_json()).expect("review packet json"),
    )
    .expect("write review packet");

    let graph_output = run_cli([
        "evidence".as_ref(),
        "graph".as_ref(),
        "--review-packet-json".as_ref(),
        review_packet_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);
    assert!(graph_output.status.success(), "{graph_output:?}");
    let evidence_graph_path = temp.path().join("evidence-graph.json");
    fs::write(&evidence_graph_path, &graph_output.stdout).expect("write evidence graph");

    let output = run_cli([
        "evidence".as_ref(),
        "assurance".as_ref(),
        "--evidence-graph-json".as_ref(),
        evidence_graph_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("assurance case json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-assurance-case"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.assurance-case"
    );
    assert_eq!(value["summary"]["cases"], 1);
    assert_eq!(value["summary"]["claims"], 1);
    assert_eq!(value["summary"]["supportedClaims"], 0);
    assert_eq!(value["summary"]["openGaps"], 1);
    assert_eq!(value["summary"]["staleItems"], 1);
    assert_eq!(value["cases"][0]["status"], "needs-review");
}

#[test]
fn cli_evidence_assurance_rejects_wrong_evidence_graph_schema_id() {
    let temp = TempDir::new().expect("temp dir");
    let review_packet_path = temp.path().join("review-packet.json");
    fs::write(
        &review_packet_path,
        serde_json::to_string(&review_packet_json()).expect("review packet json"),
    )
    .expect("write review packet");

    let graph_output = run_cli([
        "evidence".as_ref(),
        "graph".as_ref(),
        "--review-packet-json".as_ref(),
        review_packet_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);
    assert!(graph_output.status.success(), "{graph_output:?}");
    let graph_stdout = String::from_utf8(graph_output.stdout).expect("utf8 graph stdout");
    let mut graph = serde_json::from_str::<Value>(&graph_stdout).expect("evidence graph json");
    graph["schemaId"] = serde_json::json!("agent.semantic-protocols.semantic-evidence-graph.bad");
    let evidence_graph_path = temp.path().join("bad-evidence-graph.json");
    fs::write(
        &evidence_graph_path,
        serde_json::to_string(&graph).expect("evidence graph json"),
    )
    .expect("write evidence graph");

    let output = run_cli([
        "evidence".as_ref(),
        "assurance".as_ref(),
        "--evidence-graph-json".as_ref(),
        evidence_graph_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("has schemaId agent.semantic-protocols.semantic-evidence-graph.bad"),
        "{stderr}"
    );
    assert!(
        stderr.contains("expected agent.semantic-protocols.semantic-evidence-graph"),
        "{stderr}"
    );
}

#[test]
fn cli_evidence_analyze_renders_graph_turbo_request_contract() {
    let temp = TempDir::new().expect("temp dir");
    let review_packet_path = temp.path().join("review-packet.json");
    fs::write(
        &review_packet_path,
        serde_json::to_string(&review_packet_json()).expect("review packet json"),
    )
    .expect("write review packet");

    let graph_output = run_cli([
        "evidence".as_ref(),
        "graph".as_ref(),
        "--review-packet-json".as_ref(),
        review_packet_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);
    assert!(graph_output.status.success(), "{graph_output:?}");
    let evidence_graph_path = temp.path().join("evidence-graph.json");
    fs::write(&evidence_graph_path, &graph_output.stdout).expect("write evidence graph");

    let output = run_cli([
        "evidence".as_ref(),
        "analyze".as_ref(),
        "--evidence-graph-json".as_ref(),
        evidence_graph_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("analysis request json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-graph-turbo-request"
    );
    assert_eq!(value["packetKind"], "graph-turbo-request");
    assert_eq!(value["profile"], "rust-evidence-quality");
    assert_eq!(value["summary"]["graphs"], 1);
    assert_eq!(value["summary"]["nodes"], 7);
    assert_eq!(value["summary"]["gaps"], 1);
    assert_eq!(value["graphs"][0]["graphId"], "rust.evidence.graph");
    assert!(
        value["fields"]["next"]
            .as_str()
            .is_some_and(|next| next.contains("asp graph render --packet - --view seeds")),
        "{value}"
    );
}

fn review_packet_json() -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-review-packet",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.review-packet",
        "protocolVersion": "1",
        "packetId": "rust.review.packet",
        "producer": {
            "languageId": "rust",
            "providerId": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
        },
        "project": {"root": "."},
        "summary": {
            "changedInvariants": 1,
            "changedBehavior": 1,
            "missingReceipts": 1,
            "staleWaivers": 1,
            "determinismObservations": 0,
            "proofClaims": 0
        },
        "changedInvariants": [
            {
                "invariantId": "agent-r027:src.model.rs:42",
                "sourceRuleId": "AGENT-R027",
                "kind": "public-data-primitive-fields",
                "severity": "warning",
                "title": "semantic fields need named type",
                "hypothesis": "public data shape should not expose stringly fields",
                "location": {"path": "src/model.rs", "line": 42, "column": 0},
                "requiredReceipts": ["cargo-check", "expect-test"]
            }
        ],
        "changedBehavior": [
            {
                "snapshotId": "rust.behavior.src-model",
                "status": "changed",
                "subject": "src/model.rs",
                "summary": "expect-test output changed",
                "receiptIds": ["rust.expect-test.src-model"],
                "candidateIds": ["agent-r027:src.model.rs:42"]
            }
        ],
        "missingReceipts": [
            {
                "invariantId": "agent-r027:src.model.rs:42",
                "receiptKind": "expect-test",
                "reason": "no passed expect-test receipt linked to candidate"
            }
        ],
        "staleWaivers": [
            {
                "waiverId": "waiver.agent-r027.src-model",
                "invariantId": "agent-r027:src.model.rs:42",
                "receiptKind": "expect-test",
                "status": "stale",
                "owner": "reviewer",
                "reason": "snapshot migration is pending"
            }
        ],
        "reviewActions": [
            {
                "actionId": "run-receipt.agent-r027.src-model.expect-test",
                "kind": "run-receipt",
                "priority": "p0",
                "summary": "Run expect-test for agent-r027:src.model.rs:42",
                "targetId": "agent-r027:src.model.rs:42"
            }
        ]
    })
}
