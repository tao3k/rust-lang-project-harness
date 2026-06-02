use serde_json::Value;

use super::support::run_cli;

#[test]
fn cli_proof_pilot_renders_formal_proof_contract() {
    let output = run_cli([
        "proof",
        "pilot",
        "dependency-graph-acyclicity",
        "--max-nodes",
        "4",
        "--json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("proof json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-formal-proof-pilot"
    );
    assert_eq!(value["status"], "proved-bounded");
    assert_eq!(value["target"]["kind"], "dependency-graph-acyclicity");
    assert_eq!(
        value["target"]["ruleIds"],
        serde_json::json!(["AGENT-R009"])
    );
    assert_eq!(value["checks"][0]["modelsChecked"], 4166);
}
