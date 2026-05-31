use std::collections::BTreeSet;
use std::io::{self, Read};
use std::path::Path;

use serde_json::{Value, json};

use super::classify::{
    broad_raw_search_profiles, bulk_rust_read_reason, changed_check_profiles, changed_check_reason,
    command_evidence_profiles, prime_required_reason, raw_search_reason, tool_command,
    touched_file_count, touched_files_by_profile,
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

fn user_prompt_response(payload: &HookPayload, project: &ProjectProfiles) -> Option<Value> {
    let prompt = payload.prompt.as_deref().unwrap_or_default();
    if !looks_like_complex_harness_task(prompt) || project.enabled_profiles().is_empty() {
        return None;
    }
    Some(context(
        HookEvent::UserPromptSubmit,
        "Complex code task: run `rs-harness search prime --view seeds --seeds 8 .`, pick the next seed, and use subagents only for bounded `rs-harness search ... --view seeds` or `rg -n ... | rs-harness search ingest items tests .` lanes.",
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
        return Some(pre_tool_deny(&reason));
    }

    let raw_profiles = broad_raw_search_profiles(&command, policy, project);
    if !raw_profiles.is_empty() {
        return Some(pre_tool_deny(raw_search_reason(&raw_profiles)));
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
    Some(pre_tool_deny(prime_required_reason(&missing_prime)))
}

fn permission_request_response(
    payload: &HookPayload,
    policy: &CodexHookPolicy,
    project: &ProjectProfiles,
) -> Option<Value> {
    let command = tool_command(payload);
    bulk_rust_read_reason(payload, &command, policy, project)
        .map(|reason| permission_request_deny(&reason))
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

fn pre_tool_deny(reason: &str) -> Value {
    json!({
        "systemMessage": reason,
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    })
}

fn permission_request_deny(reason: &str) -> Value {
    json!({
        "systemMessage": reason,
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": {
                "behavior": "deny",
                "message": reason
            }
        }
    })
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
