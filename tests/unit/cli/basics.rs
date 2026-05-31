use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, run_cli, run_cli_with_stdin, write_clean_source, write_manifest,
};

#[test]
fn cli_renders_compact_text_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-compact");
    write_clean_source(root);

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "[ok] rust\n");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
}

#[test]
fn cli_json_flag_renders_structured_report() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-json");
    write_clean_source(root);

    let output = run_cli(["--json".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("json report");
    assert_eq!(value["modules"].as_array().expect("modules").len(), 1);
    assert_eq!(value["findings"].as_array().expect("findings").len(), 0);
}

#[test]
fn cli_agent_snapshot_renders_reasoning_tree_summary() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-agent-snapshot");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\nmod leaf;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");

    let output = run_cli(["--agent-snapshot".as_ref(), root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.starts_with("Modules:"), "{stdout}");
    assert!(stdout.contains("OwnerBranches:"), "{stdout}");
    assert!(!stdout.contains("FindingGroups:"), "{stdout}");
    assert!(!stdout.contains(" - none"), "{stdout}");
    assert!(!stdout.trim_start().starts_with('{'), "{stdout}");
    insta::assert_snapshot!("cli_agent_snapshot", stdout);
}

#[test]
fn cli_check_command_renders_policy_surface() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-check");
    write_clean_source(root);

    let compact = run_cli(["check".as_ref(), "--full".as_ref(), root.as_os_str()]);

    assert!(compact.status.success(), "{compact:?}");
    let stdout = String::from_utf8(compact.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "[ok] rust\n");

    let json = run_cli([
        "check".as_ref(),
        "--changed".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(json.status.success(), "{json:?}");
    let stdout = String::from_utf8(json.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("json report");
    assert_eq!(value["modules"].as_array().expect("modules").len(), 1);
}

#[test]
fn cli_agent_install_and_doctor_are_agent_generic() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();

    let doctor = run_cli(["agent".as_ref(), "doctor".as_ref(), root.as_os_str()]);
    assert!(doctor.status.success(), "{doctor:?}");
    let stdout = String::from_utf8(doctor.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[agent-doctor] action=checked skill=false policy=false hooks=0"),
        "{stdout}"
    );
    assert!(!stdout.contains("codex"), "{stdout}");

    let install = run_cli(["agent".as_ref(), "install".as_ref(), root.as_os_str()]);
    assert!(install.status.success(), "{install:?}");
    let stdout = String::from_utf8(install.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[agent-doctor] action=installed skill=true policy=true hooks=7"),
        "{stdout}"
    );
    assert!(root.join(".agents/skills/rs-harness/SKILL.org").exists());
    assert!(root.join(".agents/harness-policy.json").exists());
    assert!(
        root.join(".agents/hooks/agent_rs_harness_session_start.sh")
            .exists()
    );
    assert!(
        root.join(".agents/hooks/agent_rs_harness_pre_tool.sh")
            .exists()
    );
    assert!(
        root.join(".agents/hooks/agent_rs_harness_subagent_stop.sh")
            .exists()
    );
    let skill =
        fs::read_to_string(root.join(".agents/skills/rs-harness/SKILL.org")).expect("skill");
    assert!(skill.contains("rs-harness Skill"));
    assert!(skill.contains("Hooks boundary"));
    assert!(skill.contains("versionScope=external"));
    assert!(skill.contains("source=registry-source"));

    let registry = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(registry.status.success(), "{registry:?}");
    let stdout = String::from_utf8(registry.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("codex"), "{stdout}");
    let value = serde_json::from_str::<Value>(&stdout).expect("agent registry json");
    assert_eq!(
        value["registryId"],
        "agent.semantic-protocols.semantic-language-registry"
    );
    assert_eq!(value["languages"][0]["languageId"], "rust");
    assert_eq!(value["languages"][0]["providerId"], "rs-harness");
    assert_eq!(value["languages"][0]["binary"], "rs-harness");
    let methods = value["languages"][0]["methods"]
        .as_array()
        .expect("methods");
    assert!(
        methods
            .iter()
            .any(|method| method.as_str() == Some("search/deps")),
        "{value}"
    );
    assert!(
        methods
            .iter()
            .any(|method| method.as_str() == Some("agent/doctor")),
        "{value}"
    );
    assert!(
        methods
            .iter()
            .any(|method| method.as_str() == Some("agent/hook")),
        "{value}"
    );
    let descriptors = value["languages"][0]["methodDescriptors"]
        .as_array()
        .expect("method descriptors");
    assert!(
        descriptors.iter().any(|descriptor| {
            descriptor["method"] == "search/deps"
                && descriptor["command"] == "search"
                && descriptor["view"] == "deps"
                && descriptor["supportsJson"] == true
                && descriptor["supportsCompact"] == true
                && descriptor["requiresQuery"] == false
                && descriptor["acceptsStdin"] == false
                && descriptor["supportsPackageScope"] == true
        }),
        "{value}"
    );
    assert!(
        descriptors.iter().any(|descriptor| {
            descriptor["method"] == "search/ingest"
                && descriptor["command"] == "search"
                && descriptor["acceptsStdin"] == true
        }),
        "{value}"
    );
    assert!(
        descriptors.iter().any(|descriptor| {
            descriptor["method"] == "agent/doctor"
                && descriptor["command"] == "agent"
                && descriptor["supportsJson"] == true
        }),
        "{value}"
    );

    let pre_tool = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &format!(
            "{{\"hook_event_name\":\"PreToolUse\",\"cwd\":\"{}\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"rg -n \\\"timeout\\\" src tests\"}}}}",
            root.display()
        ),
    );
    assert!(pre_tool.status.success(), "{pre_tool:?}");
    let value = serde_json::from_slice::<Value>(&pre_tool.stdout).expect("pre-tool JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("search ingest")),
        "{value}"
    );

    let post_edit = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "post-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PostToolUse\",\"turn_id\":\"t1\",\"tool_name\":\"apply_patch\",\"tool_input\":{\"command\":\"*** Begin Patch\\n*** Update File: src/lib.rs\\n@@\\n+pub fn changed() {}\\n*** End Patch\\n\"}}",
    );
    assert!(post_edit.status.success(), "{post_edit:?}");
    let value = serde_json::from_slice::<Value>(&post_edit.stdout).expect("post-edit JSON");
    assert_eq!(value["decision"].as_str(), Some("block"));
    assert!(
        value["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("check --changed")),
        "{value}"
    );

    let stop_without_check = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "stop".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"Stop\",\"turn_id\":\"t1\",\"last_assistant_message\":\"draft\"}",
    );
    assert!(
        stop_without_check.status.success(),
        "{stop_without_check:?}"
    );
    let value = serde_json::from_slice::<Value>(&stop_without_check.stdout).expect("stop JSON");
    assert_eq!(value["decision"].as_str(), Some("block"));
    assert!(
        value["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("check --changed")),
        "{value}"
    );

    let changed_check = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "post-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PostToolUse\",\"turn_id\":\"t1\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"rs-harness check --changed\"}}",
    );
    assert!(changed_check.status.success(), "{changed_check:?}");
    assert!(changed_check.stdout.is_empty(), "{changed_check:?}");

    let stop_without_synthesis = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "stop".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"Stop\",\"turn_id\":\"t1\",\"last_assistant_message\":\"draft\"}",
    );
    assert!(
        stop_without_synthesis.status.success(),
        "{stop_without_synthesis:?}"
    );
    let value = serde_json::from_slice::<Value>(&stop_without_synthesis.stdout).expect("stop JSON");
    assert_eq!(value["decision"].as_str(), Some("block"));
    assert!(
        value["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("[search-synthesis]")),
        "{value}"
    );

    let stop_with_synthesis = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "stop".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"Stop\",\"turn_id\":\"t1\",\"last_assistant_message\":\"[search-synthesis] flow=hook owners=src/lib.rs findings=0 missing=- next=- edit=done\"}",
    );
    assert!(
        stop_with_synthesis.status.success(),
        "{stop_with_synthesis:?}"
    );
    assert!(
        stop_with_synthesis.stdout.is_empty(),
        "{stop_with_synthesis:?}"
    );

    let subagent_stop = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "subagent-stop".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"SubagentStop\",\"turn_id\":\"t1\",\"last_assistant_message\":\"done\"}",
    );
    assert!(subagent_stop.status.success(), "{subagent_stop:?}");
    let value = serde_json::from_slice::<Value>(&subagent_stop.stdout).expect("subagent-stop JSON");
    assert_eq!(value["decision"].as_str(), Some("block"));
    assert!(
        value["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("[search-subagent]")),
        "{value}"
    );
}

#[test]
fn cli_keeps_agent_advice_non_blocking() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-advice");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub fn public_api() {}\n",
    )
    .expect("write owned module");

    let output = run_cli([root.as_os_str()]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.contains("AGENT-R002"), "{stdout}");
    assert!(!stdout.contains("[advice]"), "{stdout}");
    assert!(!stdout.contains("No blocking issues found."), "{stdout}");
    insta::assert_snapshot!("cli_agent_advice", stdout);
}

#[test]
fn cli_exits_nonzero_for_blocking_findings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-blocking");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write lib");

    let output = run_cli([root.as_os_str()]);

    assert!(!output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    assert!(stdout.starts_with("[RUST-PROJ-R003]"), "{stdout}");
    assert!(stdout.contains("RUST-PROJ-R003"), "{stdout}");
    insta::assert_snapshot!("cli_blocking_finding", stdout);
}
