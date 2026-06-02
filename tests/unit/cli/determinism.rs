use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_determinism_readiness_renders_json_contract() {
    let temp = TempDir::new().expect("temp dir");
    let src = temp.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(
        src.join("lib.rs"),
        "pub fn sample() {\n    let _ = std::time::SystemTime::now();\n}\n",
    )
    .expect("write lib");

    let output = run_cli([
        "determinism",
        "readiness",
        "--json",
        temp.path().to_str().expect("temp path"),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("determinism json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-determinism-readiness"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.determinism-readiness"
    );
    assert_eq!(value["status"], "needs-injection");
    assert_eq!(value["observations"][0]["category"], "clock");
    assert_eq!(value["suggestions"][0]["kind"], "trait-injection");
}
