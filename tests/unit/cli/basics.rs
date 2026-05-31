use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    install_semantic_agent_hook_shim, normalize_temp_root, run_cli, run_cli_with_env,
    write_clean_source, write_manifest,
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
fn cli_agent_install_and_doctor_are_client_specific_for_codex_hooks() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "codex-hooks");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Hook fixture.\n").expect("write source");

    let doctor = run_cli(["agent".as_ref(), "doctor".as_ref(), root.as_os_str()]);
    assert!(doctor.status.success(), "{doctor:?}");
    let stdout = String::from_utf8(doctor.stdout).expect("utf8 stdout");
    assert!(stdout.contains("runtime=semantic-agent-hook"), "{stdout}");
    assert!(stdout.contains("--client codex"), "{stdout}");

    let missing_client = run_cli(["agent".as_ref(), "install".as_ref(), root.as_os_str()]);
    assert!(!missing_client.status.success(), "{missing_client:?}");
    assert!(
        String::from_utf8(missing_client.stderr)
            .expect("stderr")
            .contains("--client codex")
    );

    fs::create_dir_all(root.join(".codex")).expect("create codex dir");
    fs::write(
        root.join(".codex/config.toml"),
        r#"# User-owned Codex config.
custom_flag = "keep"
"#,
    )
    .expect("seed codex config");
    let (log_path, path) = install_semantic_agent_hook_shim(root);

    let install = run_cli_with_env(
        [
            "agent".as_ref(),
            "install".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            root.as_os_str(),
        ],
        [
            ("PATH", path.as_str()),
            (
                "SEMANTIC_AGENT_HOOK_LOG",
                log_path.to_str().expect("log path"),
            ),
        ],
    );
    assert!(install.status.success(), "{install:?}");
    let stdout = String::from_utf8(install.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with("[agent-install] client=codex"),
        "{stdout}"
    );
    assert!(
        root.join(".codex/semantic-agent-hook/profiles.rs-harness.json")
            .exists()
    );
    assert!(
        root.join(".codex/semantic-agent-hook/profiles.json")
            .exists()
    );
    let profile = serde_json::from_str::<Value>(
        &fs::read_to_string(root.join(".codex/semantic-agent-hook/profiles.rs-harness.json"))
            .expect("profile registry"),
    )
    .expect("profile JSON");
    assert_eq!(
        profile["protocolId"].as_str(),
        Some("agent.semantic-protocols.agent-hooks")
    );
    assert_eq!(profile["profiles"][0]["languageId"].as_str(), Some("rust"));
    assert_eq!(
        profile["profiles"][0]["providerId"].as_str(),
        Some("rs-harness")
    );
    assert_eq!(
        profile["profiles"][0]["commands"]["text"]["argv"].as_array(),
        Some(&vec![
            Value::String("rs-harness".to_string()),
            Value::String("search".to_string()),
            Value::String("text".to_string()),
            Value::String("{query}".to_string()),
            Value::String("tests".to_string()),
            Value::String("--view".to_string()),
            Value::String("seeds".to_string()),
            Value::String(".".to_string()),
        ])
    );
    let hooks_config =
        fs::read_to_string(root.join(".codex/config.toml")).expect("codex config.toml");
    assert!(hooks_config.contains("# BEGIN semantic-agent-hook agent hooks"));
    assert!(!hooks_config.contains("# BEGIN rs-harness agent hooks"));

    let doctor = run_cli_with_env(
        [
            "agent".as_ref(),
            "doctor".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            root.as_os_str(),
        ],
        [
            ("PATH", path.as_str()),
            (
                "SEMANTIC_AGENT_HOOK_LOG",
                log_path.to_str().expect("log path"),
            ),
        ],
    );
    assert!(doctor.status.success(), "{doctor:?}");
    let stdout = String::from_utf8(doctor.stdout).expect("doctor stdout");
    assert!(
        stdout.starts_with("[agent-doctor] status=ok client=codex"),
        "{stdout}"
    );

    let invocations = serde_json::from_str::<Value>(
        &fs::read_to_string(&log_path).expect("semantic-agent-hook invocation log"),
    )
    .expect("invocation log JSON");
    assert_eq!(invocations[0]["argv"][0].as_str(), Some("install"));
    assert_eq!(invocations[0]["argv"][3].as_str(), Some("--profiles"));
    assert_eq!(invocations[1]["argv"][0].as_str(), Some("doctor"));
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
