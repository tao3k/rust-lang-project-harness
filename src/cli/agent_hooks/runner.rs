use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;

use serde_json::{Value, json};

use super::classify::{
    broad_raw_search_profiles, bulk_rust_read_reason, changed_check_profiles, changed_check_reason,
    command_evidence_profiles, is_shell_tool, prime_required_reason, raw_search_reason,
    rust_command_guide, tool_command, touched_file_count, touched_files_by_profile,
};
use super::model::{
    HookEvent, HookPayload, Profile, hook_project_root, normalize_event, parse_hook_payload,
};
use super::policy::CodexHookPolicy;
use super::project::ProjectProfiles;
use super::state::HookState;

pub(crate) fn run_agent_hook(project_root: &Path, client: &str, event: &str) -> Result<(), String> {
    if client != "codex" {
        return Err(format!("unsupported agent hook client: {client}"));
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read agent hook stdin: {error}"))?;
    let payload = parse_hook_payload(&input)?;
    let root = hook_project_root(project_root, &payload);
    let policy = CodexHookPolicy::load(&root);
    let project = ProjectProfiles::detect(&root, &policy);
    project.save(&root)?;

    let mut state = HookState::load(&root)?;
    state.start_turn(payload.turn_id.as_deref());

    let response = match normalize_event(event, payload.hook_event_name.as_deref())? {
        HookEvent::SessionStart => {
            Some(context(HookEvent::SessionStart, project.session_context()))
        }
        HookEvent::UserPromptSubmit => user_prompt_response(&payload, &project),
        HookEvent::PreToolUse => pre_tool_response(&payload, &policy, &project, &state),
        HookEvent::PermissionRequest => permission_request_response(&payload, &policy, &project),
        HookEvent::PostToolUse => post_tool_response(&payload, &policy, &project, &mut state),
        HookEvent::SubagentStart => Some(context(
            HookEvent::SubagentStart,
            subagent_start_context(&payload),
        )),
        HookEvent::SubagentStop => subagent_stop_response(&payload, &mut state),
        HookEvent::Stop => stop_response(&payload, &policy, &state),
    };

    state.save(&root)?;
    if let Some(response) = response {
        println!(
            "{}",
            serde_json::to_string(&response)
                .map_err(|error| format!("failed to render hook response: {error}"))?
        );
    }
    Ok(())
}

pub(crate) fn run_agent_guard(
    project_root: &Path,
    client: &str,
    command_args: &[OsString],
    json_output: bool,
) -> Result<bool, String> {
    if client != "codex" {
        return Err(format!("unsupported agent guard client: {client}"));
    }
    let command = guard_command(command_args)?;
    let payload = parse_hook_payload(
        &json!({
            "hook_event_name": "PreToolUse",
            "cwd": project_root.display().to_string(),
            "tool_name": "functions.exec_command",
            "tool_input": {
                "cmd": command
            }
        })
        .to_string(),
    )?;
    let root = hook_project_root(project_root, &payload);
    let policy = CodexHookPolicy::load(&root);
    let project = ProjectProfiles::detect(&root, &policy);
    project.save(&root)?;
    let state = HookState::load(&root)?;
    if let Some(response) = pre_tool_response(&payload, &policy, &project, &state) {
        if json_output {
            println!(
                "{}",
                serde_json::to_string(&response)
                    .map_err(|error| format!("failed to render guard response: {error}"))?
            );
        } else if let Some(message) = response_decision_reason(&response)
            .or_else(|| response.get("systemMessage").and_then(Value::as_str))
        {
            eprintln!("{message}");
        }
        return Ok(false);
    }
    Ok(true)
}

fn guard_command(command_args: &[OsString]) -> Result<String, String> {
    if command_args.is_empty() {
        return Err("expected command after `--`".to_string());
    }
    command_args
        .iter()
        .map(|arg| {
            arg.to_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| "expected UTF-8 guard command argument".to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(" "))
}

pub(crate) fn print_agent_guide(project_root: &Path, client: &str) -> Result<(), String> {
    if client != "codex" {
        return Err(format!("unsupported agent guide client: {client}"));
    }
    println!(
        "{}",
        rust_command_guide("<owner-path>", &project_root.display().to_string())
    );
    Ok(())
}

fn user_prompt_response(payload: &HookPayload, project: &ProjectProfiles) -> Option<Value> {
    let prompt = payload.prompt.as_deref().unwrap_or_default();
    if !looks_like_complex_harness_task(prompt) || project.enabled_profiles().is_empty() {
        return None;
    }
    Some(context(
        HookEvent::UserPromptSubmit,
        &rust_command_guide("<owner-path>", "."),
    ))
}

fn pre_tool_response(
    payload: &HookPayload,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
    state: &HookState,
) -> Option<Value> {
    let command = tool_command(payload);
    if let Some(reason) = bulk_rust_read_reason(payload, &command, policy, project) {
        return Some(pre_tool_deny(payload, &reason));
    }

    let raw_profiles = if is_shell_tool(payload) {
        broad_raw_search_profiles(&command, policy, project)
    } else {
        BTreeSet::new()
    };
    if !raw_profiles.is_empty() {
        return Some(pre_tool_deny(payload, raw_search_reason(&raw_profiles)));
    }

    let touched = touched_files_by_profile(payload, &command, policy, project);
    let missing_prime = touched
        .keys()
        .copied()
        .filter(|profile| {
            let profile_policy = policy.profile(*profile);
            profile_policy.prime_required_before_edit && !state.profile(*profile).prime_seen
        })
        .collect::<BTreeSet<_>>();
    if missing_prime.is_empty() {
        return None;
    }
    if policy.global.exact_file_edit_exception && touched_file_count(&touched) == 1 {
        return Some(context(
            HookEvent::PreToolUse,
            "Exact-file code edit allowed before prime; run the matching profile check after editing.",
        ));
    }
    Some(pre_tool_deny(
        payload,
        prime_required_reason(&missing_prime),
    ))
}

fn permission_request_response(
    payload: &HookPayload,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
) -> Option<Value> {
    let command = tool_command(payload);
    bulk_rust_read_reason(payload, &command, policy, project)
        .map(|reason| permission_request_deny(payload, &reason))
}

fn post_tool_response(
    payload: &HookPayload,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
    state: &mut HookState,
) -> Option<Value> {
    let command = tool_command(payload);
    for profile in command_evidence_profiles(&command) {
        state.mark_prime(profile);
    }
    for profile in changed_check_profiles(&command) {
        state.mark_changed_check(profile);
    }
    for (profile, files) in touched_files_by_profile(payload, &command, policy, project) {
        state.record_dirty(profile, &files);
    }
    None
}

fn subagent_start_context(payload: &HookPayload) -> &'static str {
    let text = format!(
        "{}\n{}",
        payload.prompt.as_deref().unwrap_or_default(),
        tool_command(payload)
    );
    if text.contains("rs-harness") {
        return "Read-only Rust search subagent. Use assigned `rs-harness search ... --view seeds` or `rg -n ... | rs-harness search ingest items tests .` only; return one line `[search-subagent] role=... evidence=... missing=... next=... risk=...`.";
    }
    if text.contains("ts-harness") {
        return "Read-only TS/JS search subagent. Use only assigned ts-harness commands and return `[search-subagent] role=... evidence=... missing=... next=... risk=...`.";
    }
    "Read-only search subagent. Use parent-assigned commands only and return `[search-subagent] role=... evidence=... missing=... next=... risk=...`."
}

fn subagent_stop_response(payload: &HookPayload, state: &mut HookState) -> Option<Value> {
    if payload.stop_hook_active {
        return None;
    }
    let message = payload
        .last_assistant_message
        .as_deref()
        .unwrap_or_default();
    if message.contains("[search-subagent]")
        && message.contains("evidence=")
        && message.contains("missing=")
        && message.contains("next=")
        && message.contains("risk=")
    {
        state.subagent_results += 1;
        return None;
    }
    Some(json!({
        "decision": "block",
        "reason": "Return one compact line: `[search-subagent] role=... evidence=... missing=... next=... risk=...`."
    }))
}

fn stop_response(
    payload: &HookPayload,
    policy: &CodexHookPolicy,
    state: &HookState,
) -> Option<Value> {
    if payload.stop_hook_active {
        return None;
    }
    let dirty = [Profile::Rust, Profile::TypeScript]
        .into_iter()
        .filter(|profile| {
            let profile_state = state.profile(*profile);
            let profile_policy = policy.profile(*profile);
            profile_policy.changed_check_required
                && !profile_state.dirty_files.is_empty()
                && !profile_state.changed_check_seen
        })
        .collect::<Vec<_>>();
    (!dirty.is_empty()).then(|| {
        json!({
            "decision": "block",
            "reason": changed_check_reason(&dirty)
        })
    })
}

fn pre_tool_deny(payload: &HookPayload, reason: &str) -> Value {
    let receipt = deny_receipt(payload, reason);
    json!({
        "systemMessage": compact_system_message(reason),
        "agentHookDecision": {
            "reasonKind": receipt.reason_kind,
            "subject": receipt.subject.clone(),
            "routes": receipt.routes.clone(),
        },
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    })
}

fn permission_request_deny(payload: &HookPayload, reason: &str) -> Value {
    let receipt = deny_receipt(payload, reason);
    json!({
        "systemMessage": compact_system_message(reason),
        "agentHookDecision": {
            "reasonKind": receipt.reason_kind,
            "subject": receipt.subject.clone(),
            "routes": receipt.routes.clone(),
        },
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": {
                "behavior": "deny",
                "message": reason
            }
        }
    })
}

struct DenyReceipt {
    reason_kind: &'static str,
    subject: Value,
    routes: Value,
}

fn deny_receipt(payload: &HookPayload, reason: &str) -> DenyReceipt {
    let reason_kind = deny_reason_kind(reason);
    let paths = deny_paths(reason);
    let routes = deny_routes(reason_kind, &paths);
    DenyReceipt {
        reason_kind,
        subject: json!({
            "toolName": payload.tool_name.as_deref().unwrap_or_default(),
            "command": compact_command(&tool_command(payload)),
            "paths": paths,
        }),
        routes,
    }
}

fn compact_command(command: &str) -> String {
    const MAX_CHARS: usize = 240;
    if command.chars().count() <= MAX_CHARS {
        return command.to_string();
    }
    let preview = command.chars().take(MAX_CHARS).collect::<String>();
    format!("{preview}...<truncated>")
}

fn compact_system_message(reason: &str) -> String {
    reason.lines().next().unwrap_or(reason).to_string()
}

fn response_decision_reason(response: &Value) -> Option<&str> {
    response
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .pointer("/hookSpecificOutput/decision/message")
                .and_then(Value::as_str)
        })
}

fn deny_reason_kind(reason: &str) -> &'static str {
    if reason.contains("blocked=read-rs") {
        return "direct-source-read";
    }
    if reason.contains("blocked=bulk-rs-dump")
        || reason.contains("Raw broad Rust search")
        || reason.contains("Broad search crosses harness profiles")
    {
        return "raw-broad-search";
    }
    if reason.contains("Run search flow before editing") {
        return "prime-required";
    }
    "policy-deny"
}

fn deny_paths(reason: &str) -> Vec<String> {
    reason
        .split_whitespace()
        .filter_map(|part| part.strip_prefix("path="))
        .map(|path| {
            path.trim_matches(|character| matches!(character, '`' | '\'' | '"' | ',' | ';'))
        })
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn deny_routes(reason_kind: &str, paths: &[String]) -> Value {
    match reason_kind {
        "direct-source-read" => {
            let owner = paths.first().map(String::as_str).unwrap_or("<owner-path>");
            json!([{
                "kind": "owner",
                "argv": [
                    "rs-harness",
                    "search",
                    "owner",
                    owner,
                    "items",
                    "--trace",
                    "--view",
                    "seeds",
                    "--seeds",
                    "8",
                    "."
                ]
            }])
        }
        "raw-broad-search" => json!([{
            "kind": "ingest",
            "stdinMode": "pipe-candidates",
            "argv": [
                "rs-harness",
                "search",
                "ingest",
                "items",
                "tests",
                "--view",
                "seeds",
                "--seeds",
                "8",
                "."
            ]
        }]),
        _ => json!([]),
    }
}

fn context(event: HookEvent, message: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": event.codex_name(),
            "additionalContext": message
        }
    })
}

fn looks_like_complex_harness_task(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    [
        "refactor",
        "dependency",
        "api",
        "cargo.toml",
        "package.json",
        "tsconfig",
        "workspace",
        "parser",
        "feature",
        "cfg",
        "flow",
        "pipe",
        "search",
    ]
    .iter()
    .any(|keyword| prompt.contains(keyword))
}
