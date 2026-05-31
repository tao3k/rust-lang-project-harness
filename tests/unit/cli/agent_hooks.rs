use serde_json::{Value, json};
use tempfile::TempDir;

use super::support::{
    install_semantic_agent_hook_shim, run_cli_with_env, run_cli_with_env_and_stdin, write_manifest,
};

#[test]
fn agent_hook_delegates_payload_to_semantic_agent_hook() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-hook-bridge");
    let (log_path, path) = install_semantic_agent_hook_shim(root);
    let payload = json!({
        "hook_event_name": "PreToolUse",
        "cwd": root.display().to_string(),
        "tool_name": "functions.exec_command",
        "tool_input": {
            "cmd": "sed -n '1,80p' src/lib.rs"
        }
    })
    .to_string();

    let output = run_cli_with_env_and_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        [
            ("PATH", path.as_str()),
            (
                "SEMANTIC_AGENT_HOOK_LOG",
                log_path.to_str().expect("log path"),
            ),
        ],
        &payload,
    );

    assert!(output.status.success(), "{output:?}");
    let value = serde_json::from_slice::<Value>(&output.stdout).expect("hook JSON");
    assert_eq!(
        value["agentHookDecision"]["protocolId"].as_str(),
        Some("agent.semantic-protocols.agent-hooks")
    );
    let invocations =
        serde_json::from_str::<Value>(&std::fs::read_to_string(&log_path).expect("log"))
            .expect("invocation log");
    assert_eq!(
        invocations[0]["argv"].as_array().expect("argv"),
        &vec![
            json!("hook"),
            json!("--client"),
            json!("codex"),
            json!("pre-tool"),
            json!("--profiles"),
            json!(
                root.join(".codex/semantic-agent-hook/profiles.rs-harness.json")
                    .display()
                    .to_string()
            )
        ]
    );
    assert_eq!(invocations[0]["stdin"].as_str(), Some(payload.as_str()));
}

#[test]
fn agent_guard_delegates_to_semantic_agent_hook_decision_mode() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-guard-bridge");
    let (log_path, path) = install_semantic_agent_hook_shim(root);

    let output = run_cli_with_env(
        [
            "agent".as_ref(),
            "guard".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "--json".as_ref(),
            root.as_os_str(),
            "--".as_ref(),
            "rtk".as_ref(),
            "read".as_ref(),
            "src/lib.rs".as_ref(),
        ],
        [
            ("PATH", path.as_str()),
            (
                "SEMANTIC_AGENT_HOOK_LOG",
                log_path.to_str().expect("log path"),
            ),
        ],
    );

    assert!(!output.status.success(), "{output:?}");
    let decision = serde_json::from_slice::<Value>(&output.stdout).expect("decision JSON");
    assert_eq!(decision["decision"].as_str(), Some("deny"));
    let invocations =
        serde_json::from_str::<Value>(&std::fs::read_to_string(&log_path).expect("log"))
            .expect("invocation log");
    assert_eq!(invocations[0]["argv"][0].as_str(), Some("hook"));
    assert_eq!(invocations[0]["argv"][3].as_str(), Some("pre-tool"));
    assert_eq!(invocations[0]["argv"][6].as_str(), Some("--emit"));
    assert!(
        invocations[0]["stdin"]
            .as_str()
            .is_some_and(|stdin| stdin.contains("rtk read src/lib.rs")),
        "{invocations}"
    );
}
