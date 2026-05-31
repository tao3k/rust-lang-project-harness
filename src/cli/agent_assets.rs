//! Agent-client asset installation and doctor reporting.

use std::fs;
use std::path::Path;

use serde_json::json;

const AGENT_SKILL_CONTENT: &str = include_str!("../../skills/SKILL.org");
const CODEX_SKILL_DIR: &str = ".codex/skills/rs-harness";
const CODEX_HOOK_DIR: &str = ".codex/hooks";
const CODEX_HOOKS_CONFIG: &str = ".codex/hooks.json";
const CODEX_POLICY_PATH: &str = ".codex/harness-policy.json";
const AGENT_POLICY_CONTENT: &str = r#"{
  "profiles": {
    "rust": {
      "enabled": true,
      "prime_required_before_edit": true,
      "raw_search_requires_ingest": true,
      "changed_check_required": true
    },
    "typescript": {
      "enabled": true,
      "prime_required_before_edit": true,
      "raw_search_requires_ingest": true,
      "changed_check_required": true
    }
  },
  "global": {
    "raw_ast_grep_blocked": true,
    "exact_file_edit_exception": true,
    "docs_only_exception": true
  }
}
"#;

pub(super) fn install_agent_assets(project_root: &Path, client: &str) -> Result<(), String> {
    if client != "codex" {
        return Err(format!("unsupported agent client: {client}"));
    }
    let skill_dir = project_root.join(CODEX_SKILL_DIR);
    let hook_dir = project_root.join(CODEX_HOOK_DIR);
    let policy_path = project_root.join(CODEX_POLICY_PATH);
    let hooks_config_path = project_root.join(CODEX_HOOKS_CONFIG);
    fs::create_dir_all(&skill_dir)
        .map_err(|error| format!("failed to create agent skill dir: {error}"))?;
    fs::create_dir_all(&hook_dir)
        .map_err(|error| format!("failed to create agent hook dir: {error}"))?;
    fs::write(skill_dir.join("SKILL.org"), AGENT_SKILL_CONTENT)
        .map_err(|error| format!("failed to write agent skill: {error}"))?;
    fs::write(policy_path, AGENT_POLICY_CONTENT)
        .map_err(|error| format!("failed to write agent hook policy: {error}"))?;
    fs::write(hooks_config_path, agent_hooks_config()?)
        .map_err(|error| format!("failed to write codex hooks config: {error}"))?;
    for hook in agent_hook_assets() {
        write_agent_hook(
            &hook_dir.join(hook.file_name),
            &agent_hook_script(hook.event),
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct AgentHookAsset {
    label: &'static str,
    event: &'static str,
    file_name: &'static str,
}

fn agent_hook_assets() -> &'static [AgentHookAsset] {
    &[
        AgentHookAsset {
            label: "session-start",
            event: "session-start",
            file_name: "agent_rs_harness_codex_session_start.sh",
        },
        AgentHookAsset {
            label: "user-prompt",
            event: "user-prompt",
            file_name: "agent_rs_harness_codex_user_prompt.sh",
        },
        AgentHookAsset {
            label: "pre-tool",
            event: "pre-tool",
            file_name: "agent_rs_harness_codex_pre_tool.sh",
        },
        AgentHookAsset {
            label: "post-tool",
            event: "post-tool",
            file_name: "agent_rs_harness_codex_post_tool.sh",
        },
        AgentHookAsset {
            label: "subagent-start",
            event: "subagent-start",
            file_name: "agent_rs_harness_codex_subagent_start.sh",
        },
        AgentHookAsset {
            label: "subagent-stop",
            event: "subagent-stop",
            file_name: "agent_rs_harness_codex_subagent_stop.sh",
        },
        AgentHookAsset {
            label: "stop",
            event: "stop",
            file_name: "agent_rs_harness_codex_stop.sh",
        },
    ]
}

fn agent_hook_script(event: &str) -> String {
    format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\nrepo_root=\"$(git rev-parse --show-toplevel 2>/dev/null || pwd)\"\ncd \"$repo_root\"\nexec rs-harness agent hook --client codex {event} \"$repo_root\"\n"
    )
}

fn agent_hooks_config() -> Result<String, String> {
    let command = |file_name: &str| {
        format!("bash \"$(git rev-parse --show-toplevel)/{CODEX_HOOK_DIR}/{file_name}\"")
    };
    let hook = |file_name: &str, status_message: &str| {
        json!({
            "type": "command",
            "command": command(file_name),
            "statusMessage": status_message
        })
    };
    let value = json!({
        "hooks": {
            "SessionStart": [
                {
                    "matcher": "startup|resume",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_session_start.sh",
                            "Loading rs-harness search protocol"
                        )
                    ]
                }
            ],
            "UserPromptSubmit": [
                {
                    "matcher": ".*",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_user_prompt.sh",
                            "Checking rs-harness prompt guidance"
                        )
                    ]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "Bash|apply_patch|Edit|Write",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_pre_tool.sh",
                            "Checking rs-harness search flow"
                        )
                    ]
                }
            ],
            "PostToolUse": [
                {
                    "matcher": "Bash|apply_patch|Edit|Write",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_post_tool.sh",
                            "Updating rs-harness search flow state"
                        )
                    ]
                }
            ],
            "SubagentStart": [
                {
                    "matcher": ".*",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_subagent_start.sh",
                            "Preparing rs-harness subagent context"
                        )
                    ]
                }
            ],
            "SubagentStop": [
                {
                    "matcher": ".*",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_subagent_stop.sh",
                            "Checking rs-harness subagent evidence"
                        )
                    ]
                }
            ],
            "Stop": [
                {
                    "matcher": ".*",
                    "hooks": [
                        hook(
                            "agent_rs_harness_codex_stop.sh",
                            "Checking rs-harness changed files"
                        )
                    ]
                }
            ]
        }
    });
    serde_json::to_string_pretty(&value)
        .map(|content| format!("{content}\n"))
        .map_err(|error| format!("failed to serialize codex hooks config: {error}"))
}

fn write_agent_hook(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|error| format!("failed to write agent hook: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|error| format!("failed to stat agent hook: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|error| format!("failed to chmod agent hook: {error}"))?;
    }
    Ok(())
}

pub(super) fn print_agent_doctor(
    project_root: &Path,
    action: &str,
    client: Option<&str>,
) -> Result<(), String> {
    let Some(client) = client else {
        println!(
            "[agent-doctor] action={action} client=none skill=false policy=false config=false hooks=0 daemon=missing protocol=cli-cold"
        );
        println!(
            "|note kind=client-required message=\"pass --client codex to inspect Codex assets\""
        );
        return Ok(());
    };
    if client != "codex" {
        return Err(format!("unsupported agent client: {client}"));
    }
    let skill = project_root.join(CODEX_SKILL_DIR).join("SKILL.org");
    let policy = project_root.join(CODEX_POLICY_PATH);
    let hooks_config = project_root.join(CODEX_HOOKS_CONFIG);
    let hook_paths = agent_hook_assets()
        .iter()
        .map(|hook| {
            (
                hook.label,
                project_root.join(CODEX_HOOK_DIR).join(hook.file_name),
            )
        })
        .collect::<Vec<_>>();
    println!(
        "[agent-doctor] action={action} client=codex skill={} policy={} config={} hooks={} daemon=missing protocol=cli-cold",
        skill.exists(),
        policy.exists(),
        hooks_config.exists(),
        hook_paths.iter().filter(|(_, path)| path.exists()).count()
    );
    println!(
        "|skill {} present={}",
        display_cli_path(project_root, &skill),
        skill.exists()
    );
    println!(
        "|policy {} present={}",
        display_cli_path(project_root, &policy),
        policy.exists()
    );
    println!(
        "|config {} present={}",
        display_cli_path(project_root, &hooks_config),
        hooks_config.exists()
    );
    for (label, path) in hook_paths {
        println!(
            "|hook {label} path={} present={}",
            display_cli_path(project_root, &path),
            path.exists()
        );
    }
    Ok(())
}

fn display_cli_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
