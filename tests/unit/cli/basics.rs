use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, run_cli, run_cli_with_env, run_cli_with_stdin, write_clean_source,
    write_manifest,
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
    assert!(
        stdout.starts_with(
            "[agent-doctor] action=checked client=none skill=false policy=false config=false hooks=0"
        ),
        "{stdout}"
    );
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

# BEGIN ts-harness agent hooks
[[hooks.PreToolUse]]
matcher = ".*(Read|exec_command).*"

[[hooks.PreToolUse.hooks]]
type = "command"
timeout = 5
statusMessage = "Checking ts-harness search flow"
command = '''
exec ts-harness agent hook --client codex pre-tool "$PWD"
'''
# END ts-harness agent hooks
"#,
    )
    .expect("seed codex config");

    let install = run_cli([
        "agent".as_ref(),
        "install".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(install.status.success(), "{install:?}");
    let stdout = String::from_utf8(install.stdout).expect("utf8 stdout");
    assert!(
        stdout.starts_with(
            "[agent-doctor] action=installed client=codex skill=true policy=true config=true hooks=8"
        ),
        "{stdout}"
    );
    assert!(root.join(".codex/skills/rs-harness/SKILL.org").exists());
    assert!(root.join(".codex/harness-policy.json").exists());
    assert!(root.join(".codex/config.toml").exists());
    assert!(!root.join(".codex/hooks.json").exists());
    let hooks_config =
        fs::read_to_string(root.join(".codex/config.toml")).expect("codex config.toml");
    assert!(hooks_config.contains("# User-owned Codex config."));
    assert!(hooks_config.contains("custom_flag = \"keep\""));
    assert!(hooks_config.contains("# BEGIN ts-harness agent hooks"));
    assert!(hooks_config.contains("ts-harness agent hook --client codex pre-tool"));
    assert!(hooks_config.contains("# END ts-harness agent hooks"));
    assert!(hooks_config.contains("# BEGIN rs-harness agent hooks"));
    assert!(hooks_config.contains("# END rs-harness agent hooks"));
    assert_eq!(
        hooks_config
            .matches("# BEGIN rs-harness agent hooks")
            .count(),
        1
    );
    assert_eq!(
        hooks_config
            .matches("# BEGIN ts-harness agent hooks")
            .count(),
        1
    );
    assert!(hooks_config.contains("[[hooks.SessionStart]]"));
    assert!(hooks_config.contains("[[hooks.UserPromptSubmit]]"));
    assert!(hooks_config.contains("[[hooks.PermissionRequest]]"));
    assert!(hooks_config.contains("[[hooks.SubagentStart]]"));
    assert!(hooks_config.contains("[[hooks.Stop]]"));
    assert!(hooks_config.contains("unified_exec = true"));
    assert!(hooks_config.contains(".*(Read|readFile|readDirectory|read_file"));
    assert!(hooks_config.contains("readFile"));
    assert!(hooks_config.contains("readDirectory"));
    assert!(hooks_config.contains("FsReadFile"));
    assert!(hooks_config.contains("FsReadDirectory"));
    assert!(hooks_config.contains("fs/readFile"));
    assert!(hooks_config.contains("fs/readDirectory"));
    assert!(hooks_config.contains("fs\\\\.read"));
    assert!(hooks_config.contains("fs\\\\.readbin"));
    assert!(hooks_config.contains("command_execution"));
    assert!(hooks_config.contains("Bash|exec_command|command_execution"));
    assert!(hooks_config.contains("rs-harness agent hook --client codex pre-tool"));
    assert!(hooks_config.contains("rs-harness agent hook --client codex permission-request"));

    let reinstall = run_cli([
        "agent".as_ref(),
        "install".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(reinstall.status.success(), "{reinstall:?}");
    let reinstalled_config =
        fs::read_to_string(root.join(".codex/config.toml")).expect("reinstalled codex config");
    assert_eq!(
        reinstalled_config
            .matches("# BEGIN rs-harness agent hooks")
            .count(),
        1
    );
    assert_eq!(
        reinstalled_config
            .matches("# BEGIN ts-harness agent hooks")
            .count(),
        1
    );
    assert!(reinstalled_config.contains("ts-harness agent hook --client codex pre-tool"));
    assert!(reinstalled_config.contains("custom_flag = \"keep\""));

    let codex_home = temp.path().join("codex-home");
    let profile_install = run_cli_with_env(
        [
            "agent".as_ref(),
            "install".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "--scope".as_ref(),
            "profile".as_ref(),
            "--profile".as_ref(),
            "rs-harness-ci".as_ref(),
            root.as_os_str(),
        ],
        [("CODEX_HOME", codex_home.as_os_str())],
    );
    assert!(profile_install.status.success(), "{profile_install:?}");
    let stdout = String::from_utf8(profile_install.stdout).expect("profile install stdout");
    assert!(
        stdout.starts_with(
            "[agent-doctor] action=installed client=codex scope=profile profile=rs-harness-ci skill=true policy=true config=true hooks=8"
        ),
        "{stdout}"
    );
    let profile_config_path = codex_home.join("rs-harness-ci.config.toml");
    assert!(profile_config_path.exists());
    let profile_config = fs::read_to_string(&profile_config_path).expect("profile config");
    assert!(profile_config.contains("unified_exec = true"));
    assert!(profile_config.contains("# BEGIN rs-harness agent hooks"));
    assert!(profile_config.contains("command_execution"));
    assert!(profile_config.contains("rs-harness agent hook --client codex pre-tool"));
    assert!(!profile_config.contains("ts-harness agent hook --client codex pre-tool"));

    let profile_doctor = run_cli_with_env(
        [
            "agent".as_ref(),
            "doctor".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "--scope".as_ref(),
            "profile".as_ref(),
            "--profile".as_ref(),
            "rs-harness-ci".as_ref(),
            root.as_os_str(),
        ],
        [("CODEX_HOME", codex_home.as_os_str())],
    );
    assert!(profile_doctor.status.success(), "{profile_doctor:?}");
    let stdout = String::from_utf8(profile_doctor.stdout).expect("profile doctor stdout");
    assert!(stdout.contains("|profile name=rs-harness-ci"));
    assert!(stdout.contains("codex --profile rs-harness-ci exec -C <repo>"));
    assert!(stdout.contains("|compat unified_exec present=true required=true"));
    assert!(stdout.contains("|compat matcher namespaced=true required=true"));

    let skill = fs::read_to_string(root.join(".codex/skills/rs-harness/SKILL.org")).expect("skill");
    assert!(skill.contains("rs-harness Skill"));
    assert!(skill.contains("Hooks boundary"));
    assert!(skill.contains("Profile selection"));
    assert!(skill.contains("versionScope=external"));
    assert!(skill.contains("source=registry-source"));

    let stale_config = hooks_config.replace("unified_exec = true\n\n", "");
    fs::write(root.join(".codex/config.toml"), stale_config).expect("write stale codex config");
    let stale_doctor = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(stale_doctor.status.success(), "{stale_doctor:?}");
    let stdout = String::from_utf8(stale_doctor.stdout).expect("stale doctor stdout");
    assert!(stdout.contains("|compat unified_exec present=false required=true"));
    assert!(stdout.contains("|warn kind=codex-unified-exec"));
    fs::write(root.join(".codex/config.toml"), &hooks_config).expect("restore codex config");

    let disabled_pretool_config = hooks_config.replace(
        "matcher = \".*(Read|readFile|readDirectory|read_file|FsReadFile|FsReadDirectory|fs\\\\.read|fs\\\\.readDirectory|fs/readFile|fs/readDirectory|fs\\\\.readbin|writeFile|FsWriteFile|fs\\\\.write|fs/write|fs\\\\.writeFile|fs/writeFile|FsRemove|fs\\\\.remove|fs/remove|FsCopy|fs\\\\.copy|fs/copy|fs\\\\.rename|fs/rename|mcp__.*__read.*|Bash|exec_command|command_execution|apply_patch|Edit|Write).*\"",
        "matcher = \"__rs_harness_pretool_disabled_during_hook_upgrade__\"",
    );
    fs::write(root.join(".codex/config.toml"), disabled_pretool_config)
        .expect("write disabled pretool config");
    let matcher_doctor = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(matcher_doctor.status.success(), "{matcher_doctor:?}");
    let stdout = String::from_utf8(matcher_doctor.stdout).expect("matcher doctor stdout");
    assert!(stdout.contains("|compat matcher namespaced=false required=true"));
    assert!(stdout.contains("|warn kind=codex-tool-matcher"));
    fs::write(root.join(".codex/config.toml"), &hooks_config).expect("restore codex config");

    let registry = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(registry.status.success(), "{registry:?}");
    let stdout = String::from_utf8(registry.stdout).expect("utf8 stdout");
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
            .any(|method| method.as_str() == Some("agent/guide")),
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
    assert!(
        descriptors.iter().any(|descriptor| {
            descriptor["method"] == "agent/guide"
                && descriptor["command"] == "agent"
                && descriptor["supportsJson"] == false
        }),
        "{value}"
    );

    let guide = run_cli([
        "agent".as_ref(),
        "guide".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
    ]);
    assert!(guide.status.success(), "{guide:?}");
    let guide_stdout = String::from_utf8(guide.stdout).expect("guide stdout");
    assert!(
        guide_stdout.starts_with("|flow guide=prime->batch-or-owner->tests->edit"),
        "{guide_stdout}"
    );
    assert!(guide_stdout.contains("|prime run=`rs-harness search prime --view seeds --seeds 8"));
    assert!(
        guide_stdout.contains("|batch run=`printf '%s\\n' <paths...> | rs-harness search ingest")
    );
    assert!(guide_stdout.contains("|owner run=`rs-harness search owner <owner-path> items"));
    assert!(guide_stdout.contains("installed-binary-only"));
    assert!(guide_stdout.contains("[search-subagent] role="));
    assert!(!guide_stdout.contains("README"), "{guide_stdout}");
    assert!(!guide_stdout.contains("SKILL"), "{guide_stdout}");
    assert!(!guide_stdout.contains("docs/"), "{guide_stdout}");

    let pre_tool = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "Bash",
            "tool_input": {
                "command": "rg -n \"timeout\" --glob '*.rs' src tests | rs-harness search ingest items tests --view seeds --seeds 8 ."
            }
        })
        .to_string(),
    );
    assert!(pre_tool.status.success(), "{pre_tool:?}");
    assert!(pre_tool.stdout.is_empty(), "{pre_tool:?}");

    let exec_command_pre_tool = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "exec_command",
            "tool_input": {
                "cmd": "rg -n \"timeout\" README.md docs --glob '*.md'"
            }
        })
        .to_string(),
    );
    assert!(
        exec_command_pre_tool.status.success(),
        "{exec_command_pre_tool:?}"
    );
    assert!(
        exec_command_pre_tool.stdout.is_empty(),
        "{exec_command_pre_tool:?}"
    );

    let raw_rust_rg = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "Bash",
            "tool_input": {
                "command": "rg -n \"timeout\" src tests"
            }
        })
        .to_string(),
    );
    assert!(raw_rust_rg.status.success(), "{raw_rust_rg:?}");
    let value = serde_json::from_slice::<Value>(&raw_rust_rg.stdout).expect("raw rg JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("raw-broad-search")
    );
    assert_eq!(
        value["agentHookDecision"]["routes"][0]["kind"].as_str(),
        Some("ingest")
    );
    assert_eq!(
        value["agentHookDecision"]["routes"][0]["stdinMode"].as_str(),
        Some("pipe-candidates")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=bulk-rs-dump")
                    && reason.contains(
                        "rg -n \"<query>\" --glob '*.rs' src tests | rs-harness search ingest",
                    )
                    && reason.contains("pipe-to-ingest")
            }),
        "{value}"
    );

    let raw_workspace_search = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": "rg -n \"timeout\" ."
            }
        })
        .to_string(),
    );
    assert!(
        raw_workspace_search.status.success(),
        "{raw_workspace_search:?}"
    );
    let value = serde_json::from_slice::<Value>(&raw_workspace_search.stdout)
        .expect("raw workspace search JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("raw-broad-search")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Raw broad Rust search")),
        "{value}"
    );

    let rtk_grep = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": "rtk grep -n timeout src tests"
            }
        })
        .to_string(),
    );
    assert!(rtk_grep.status.success(), "{rtk_grep:?}");
    let value = serde_json::from_slice::<Value>(&rtk_grep.stdout).expect("rtk grep JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("raw-broad-search")
    );
    assert_eq!(
        value["agentHookDecision"]["routes"][0]["kind"].as_str(),
        Some("ingest")
    );
    assert_eq!(
        value["agentHookDecision"]["routes"][0]["stdinMode"].as_str(),
        Some("pipe-candidates")
    );

    let context_rust_rg = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "Bash",
            "tool_input": {
                "command": "rg -n \"fn load\" src/lib.rs -A 28 -B 4"
            }
        })
        .to_string(),
    );
    assert!(context_rust_rg.status.success(), "{context_rust_rg:?}");
    let value = serde_json::from_slice::<Value>(&context_rust_rg.stdout).expect("context rg JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("raw-broad-search")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=bulk-rs-dump")
                    && reason.contains("flow guide=prime->rg-or-paths->ingest->owner-or-deps")
                    && reason.contains("rs-harness search ingest items tests")
            }),
        "{value}"
    );

    let command_patch_pre_tool = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "apply_patch",
            "tool_input": {
                "command": "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n+pub fn patched() {}\n*** End Patch\n"
            }
        })
        .to_string(),
    );
    assert!(
        command_patch_pre_tool.status.success(),
        "{command_patch_pre_tool:?}"
    );
    let stdout = String::from_utf8(command_patch_pre_tool.stdout).expect("patch pre-tool stdout");
    assert!(!stdout.contains("blocked=bulk-rs-dump"), "{stdout}");
    assert!(!stdout.contains("blocked=read-rs"), "{stdout}");
    assert!(stdout.contains("Exact-file code edit allowed"), "{stdout}");

    let filesystem_write_file = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "fs/writeFile",
            "tool_input": {
                "path": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(
        filesystem_write_file.status.success(),
        "{filesystem_write_file:?}"
    );
    let stdout = String::from_utf8(filesystem_write_file.stdout).expect("filesystem write stdout");
    assert!(!stdout.contains("blocked=read-rs"), "{stdout}");
    assert!(stdout.contains("Exact-file code edit allowed"), "{stdout}");

    let direct_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "Read",
            "tool_input": {
                "path": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(direct_read.status.success(), "{direct_read:?}");
    let value = serde_json::from_slice::<Value>(&direct_read.stdout).expect("pre-tool JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=read-rs")
                    && reason.contains("route=owner")
                    && reason.contains(
                        "rs-harness search owner src/lib.rs items --trace --view seeds --seeds 8",
                    )
                    && reason.contains("one-search-command-at-a-time")
                    && !reason.contains("target/debug/rs-harness")
                    && reason.contains("agentHookDecision.routes")
            }),
        "{value}"
    );

    let filesystem_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "fs.read",
            "tool_input": {
                "filePath": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(filesystem_read.status.success(), "{filesystem_read:?}");
    let value = serde_json::from_slice::<Value>(&filesystem_read.stdout).expect("fs read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("direct-source-read")
    );
    assert_eq!(
        value["agentHookDecision"]["subject"]["paths"][0].as_str(),
        Some("src/lib.rs")
    );
    assert_eq!(
        value["agentHookDecision"]["routes"][0]["kind"].as_str(),
        Some("owner")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("blocked=read-rs path=src/lib.rs")),
        "{value}"
    );

    let filesystem_read_file = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "fs.readFile",
            "tool_input": {
                "filePath": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(
        filesystem_read_file.status.success(),
        "{filesystem_read_file:?}"
    );
    let value =
        serde_json::from_slice::<Value>(&filesystem_read_file.stdout).expect("fs readFile JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("blocked=read-rs path=src/lib.rs")),
        "{value}"
    );

    let exec_server_read_file = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "fs/readFile",
            "tool_input": {
                "path": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(
        exec_server_read_file.status.success(),
        "{exec_server_read_file:?}"
    );
    let value = serde_json::from_slice::<Value>(&exec_server_read_file.stdout)
        .expect("exec-server readFile JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );

    let filesystem_read_directory = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "FsReadDirectory",
            "tool_input": {
                "path": "src"
            }
        })
        .to_string(),
    );
    assert!(
        filesystem_read_directory.status.success(),
        "{filesystem_read_directory:?}"
    );
    assert!(
        filesystem_read_directory.stdout.is_empty(),
        "{filesystem_read_directory:?}"
    );

    let read_without_path_key = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "Read",
            "tool_input": {
                "note": "src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(
        read_without_path_key.status.success(),
        "{read_without_path_key:?}"
    );
    assert!(
        read_without_path_key.stdout.is_empty(),
        "{read_without_path_key:?}"
    );

    let rtk_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": "rtk read src/lib.rs"
            }
        })
        .to_string(),
    );
    assert!(rtk_read.status.success(), "{rtk_read:?}");
    let value = serde_json::from_slice::<Value>(&rtk_read.stdout).expect("rtk read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("direct-source-read")
    );
    assert_eq!(
        value["agentHookDecision"]["subject"]["paths"][0].as_str(),
        Some("src/lib.rs")
    );

    let nested_rtk_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "multi_tool_use.parallel",
            "tool_input": {
                "tool_uses": [{
                    "recipient_name": "functions.exec_command",
                    "parameters": {
                        "cmd": "rtk read src/lib.rs"
                    }
                }]
            }
        })
        .to_string(),
    );
    assert!(nested_rtk_read.status.success(), "{nested_rtk_read:?}");
    let value =
        serde_json::from_slice::<Value>(&nested_rtk_read.stdout).expect("nested rtk read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("direct-source-read")
    );
    assert_eq!(
        value["agentHookDecision"]["subject"]["paths"][0].as_str(),
        Some("src/lib.rs")
    );

    let bulk_shell_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"rg --files -g '*.rs' | xargs sed -n '1,40p'\"}}",
    );
    assert!(bulk_shell_read.status.success(), "{bulk_shell_read:?}");
    let value = serde_json::from_slice::<Value>(&bulk_shell_read.stdout).expect("pre-tool JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=bulk-rs-dump")
                    && reason.contains("flow guide=prime->rg-or-paths->ingest->owner-or-deps")
                    && reason.contains("pipe-to-ingest")
                    && reason.contains("rs-harness search deps <dep[/path][::api]> public-api")
                    && reason.contains("one-search-command-at-a-time")
                    && reason.contains("[search-subagent]")
            }),
        "{value}"
    );

    let shell_single_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"sed -n '1,120p' src/lib.rs\"}}",
    );
    assert!(shell_single_read.status.success(), "{shell_single_read:?}");
    let value =
        serde_json::from_slice::<Value>(&shell_single_read.stdout).expect("shell single read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=read-rs path=src/lib.rs")
                    && reason.contains("rs-harness search owner src/lib.rs items")
                    && reason.contains("route=owner")
            }),
        "{value}"
    );

    let command_execution_command =
        ["/bin/zsh -lc \"", "s", "ed -n '1,8p' src/lib.rs", "\""].concat();
    let command_execution_payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "command_execution",
        "tool_input": { "command": command_execution_command }
    })
    .to_string();
    let command_execution_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &command_execution_payload,
    );
    assert!(
        command_execution_read.status.success(),
        "{command_execution_read:?}"
    );
    let value = serde_json::from_slice::<Value>(&command_execution_read.stdout)
        .expect("command execution JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );

    let namespaced_python_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": "python -c \"print(open('src/lib.rs').readline())\""
            }
        })
        .to_string(),
    );
    assert!(
        namespaced_python_read.status.success(),
        "{namespaced_python_read:?}"
    );
    let value =
        serde_json::from_slice::<Value>(&namespaced_python_read.stdout).expect("python read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| {
                reason.contains("[rs-harness-flow] blocked=read-rs path=src/lib.rs")
                    && reason.contains("rs-harness search owner src/lib.rs items")
            }),
        "{value}"
    );

    let node_read = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"functions.exec_command\",\"tool_input\":{\"cmd\":\"node -e \\\"require('fs').readFileSync('src/lib.rs','utf8')\\\"\"}}",
    );
    assert!(node_read.status.success(), "{node_read:?}");
    let value = serde_json::from_slice::<Value>(&node_read.stdout).expect("node read JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );

    let permission_request = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "permission-request".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PermissionRequest\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"cat src/*.rs\"}}",
    );
    assert!(
        permission_request.status.success(),
        "{permission_request:?}"
    );
    let value = serde_json::from_slice::<Value>(&permission_request.stdout)
        .expect("permission-request JSON");
    assert_eq!(
        value["hookSpecificOutput"]["decision"]["behavior"].as_str(),
        Some("deny")
    );
    assert!(
        value["hookSpecificOutput"]["decision"]["message"]
            .as_str()
            .is_some_and(|reason| reason.contains("rs-harness search ingest items tests")),
        "{value}"
    );

    let docs_search = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"rg -n \\\"timeout\\\" README.md docs/\"}}",
    );
    assert!(docs_search.status.success(), "{docs_search:?}");
    assert!(docs_search.stdout.is_empty(), "{docs_search:?}");

    let non_rust_source_search = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &serde_json::json!({
            "hook_event_name": "PreToolUse",
            "cwd": root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": "rg -n \"timeout\" src --glob '!*.rs'"
            }
        })
        .to_string(),
    );
    assert!(
        non_rust_source_search.status.success(),
        "{non_rust_source_search:?}"
    );
    assert!(
        non_rust_source_search.stdout.is_empty(),
        "{non_rust_source_search:?}"
    );

    let config_command = ["g", "rep -n \"unified_exec\" .codex/config.toml"].concat();
    let config_payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "functions.exec_command",
        "tool_input": { "cmd": config_command }
    })
    .to_string();
    let config_search = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        &config_payload,
    );
    assert!(config_search.status.success(), "{config_search:?}");
    assert!(config_search.stdout.is_empty(), "{config_search:?}");

    let exact_file_search = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "pre-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"rg -n \\\"timeout\\\" src/lib.rs\"}}",
    );
    assert!(exact_file_search.status.success(), "{exact_file_search:?}");
    assert!(exact_file_search.stdout.is_empty(), "{exact_file_search:?}");

    let guard_rtk_read = run_cli([
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
    ]);
    assert!(!guard_rtk_read.status.success(), "{guard_rtk_read:?}");
    let value = serde_json::from_slice::<Value>(&guard_rtk_read.stdout).expect("guard JSON");
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        value["agentHookDecision"]["reasonKind"].as_str(),
        Some("direct-source-read")
    );

    let guard_exact_file_search = run_cli([
        "agent".as_ref(),
        "guard".as_ref(),
        "--client".as_ref(),
        "codex".as_ref(),
        root.as_os_str(),
        "--".as_ref(),
        "rg".as_ref(),
        "-n".as_ref(),
        "timeout".as_ref(),
        "src/lib.rs".as_ref(),
    ]);
    assert!(
        guard_exact_file_search.status.success(),
        "{guard_exact_file_search:?}"
    );
    assert!(
        guard_exact_file_search.stdout.is_empty(),
        "{guard_exact_file_search:?}"
    );

    let post_edit = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "post-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PostToolUse\",\"turn_id\":\"t1\",\"tool_name\":\"apply_patch\",\"tool_input\":{\"command\":\"*** Begin Patch\\n*** Update File: src/lib.rs\\n@@\\n+pub fn changed() {}\\n*** End Patch\\n\"}}",
    );
    assert!(post_edit.status.success(), "{post_edit:?}");
    assert!(post_edit.stdout.is_empty(), "{post_edit:?}");

    let stop_without_check = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
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
            "--client".as_ref(),
            "codex".as_ref(),
            "post-tool".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"PostToolUse\",\"turn_id\":\"t1\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"rs-harness check --changed\"}}",
    );
    assert!(changed_check.status.success(), "{changed_check:?}");
    assert!(changed_check.stdout.is_empty(), "{changed_check:?}");

    let stop_after_check = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
            "stop".as_ref(),
            root.as_os_str(),
        ],
        "{\"hook_event_name\":\"Stop\",\"turn_id\":\"t1\",\"last_assistant_message\":\"draft\"}",
    );
    assert!(stop_after_check.status.success(), "{stop_after_check:?}");
    assert!(stop_after_check.stdout.is_empty(), "{stop_after_check:?}");

    let subagent_stop = run_cli_with_stdin(
        [
            "agent".as_ref(),
            "hook".as_ref(),
            "--client".as_ref(),
            "codex".as_ref(),
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
