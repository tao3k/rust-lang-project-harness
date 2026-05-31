//! Generic agent hook policy for installed harness assets.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const POLICY_PATH: &str = ".agents/harness-policy.json";
const STATE_DIR: &str = ".agents/harness-state";

pub(super) fn run_agent_hook(project_root: &Path, event: &str) -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read agent hook stdin: {error}"))?;
    let payload = parse_hook_payload(&input)?;
    let root = hook_project_root(project_root, &payload);
    let policy = HookPolicy::load(&root);
    let mut state = HookState::load(&root, payload.session_id.as_deref())?;
    state.start_turn(payload.turn_id.as_deref());

    let response = match normalize_event(event, payload.hook_event_name.as_deref())? {
        HookEvent::SessionStart => Some(context(
            HookEvent::SessionStart,
            "Use harness search. For complex Rust tasks, run `rs-harness search prime` before broad reading or editing.",
        )),
        HookEvent::UserPromptSubmit => user_prompt_response(&payload),
        HookEvent::PreToolUse => pre_tool_response(&payload, &policy, &state),
        HookEvent::PostToolUse => post_tool_response(&root, &payload, &policy, &mut state),
        HookEvent::SubagentStart => Some(context(
            HookEvent::SubagentStart,
            "Read-only search subagent: run only assigned harness searches and return `[search-subagent] role=... evidence=... missing=... next=... risk=...`.",
        )),
        HookEvent::SubagentStop => subagent_stop_response(&payload, &policy, &mut state),
        HookEvent::Stop => stop_response(&payload, &policy, &mut state),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookEvent {
    SessionStart,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    SubagentStart,
    SubagentStop,
    Stop,
}

impl HookEvent {
    fn codex_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::Stop => "Stop",
        }
    }
}

fn normalize_event(explicit: &str, payload_event: Option<&str>) -> Result<HookEvent, String> {
    let event = if explicit.is_empty() {
        payload_event.unwrap_or("")
    } else {
        explicit
    };
    match event {
        "session-start" | "SessionStart" => Ok(HookEvent::SessionStart),
        "user-prompt" | "UserPromptSubmit" => Ok(HookEvent::UserPromptSubmit),
        "pre-tool" | "PreToolUse" => Ok(HookEvent::PreToolUse),
        "post-tool" | "PostToolUse" => Ok(HookEvent::PostToolUse),
        "subagent-start" | "SubagentStart" => Ok(HookEvent::SubagentStart),
        "subagent-stop" | "SubagentStop" => Ok(HookEvent::SubagentStop),
        "stop" | "Stop" => Ok(HookEvent::Stop),
        other => Err(format!("unknown agent hook event: {other}")),
    }
}

#[derive(Debug, Deserialize)]
struct HookPayload {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    turn_id: Option<String>,
    #[serde(default)]
    hook_event_name: Option<String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Value,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    last_assistant_message: Option<String>,
    #[serde(default)]
    stop_hook_active: bool,
}

fn parse_hook_payload(input: &str) -> Result<HookPayload, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        serde_json::from_value(json!({}))
    } else {
        serde_json::from_str(trimmed)
    }
    .map_err(|error| format!("failed to parse agent hook JSON: {error}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct HookPolicy {
    mode: HookMode,
    prime_required_before_edit: bool,
    raw_rg_requires_ingest: bool,
    raw_fd_requires_ingest: bool,
    raw_ast_grep_blocked: bool,
    changed_check_required_after_edit: bool,
    subagent_evidence_required: bool,
    synthesis_required_after_edit: bool,
    exact_file_edit_exception: bool,
}

impl Default for HookPolicy {
    fn default() -> Self {
        Self {
            mode: HookMode::Strict,
            prime_required_before_edit: true,
            raw_rg_requires_ingest: true,
            raw_fd_requires_ingest: true,
            raw_ast_grep_blocked: true,
            changed_check_required_after_edit: true,
            subagent_evidence_required: true,
            synthesis_required_after_edit: true,
            exact_file_edit_exception: true,
        }
    }
}

impl HookPolicy {
    fn load(root: &Path) -> Self {
        let path = root.join(POLICY_PATH);
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    fn blocks(&self) -> bool {
        matches!(self.mode, HookMode::Strict | HookMode::Ci)
    }

    fn nudges(&self) -> bool {
        !matches!(self.mode, HookMode::Observe)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum HookMode {
    Observe,
    Nudge,
    #[default]
    Strict,
    Ci,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HookState {
    #[serde(default)]
    turn_id: Option<String>,
    #[serde(default)]
    prime_seen: bool,
    #[serde(default)]
    ingest_seen: bool,
    #[serde(default)]
    changed_check_seen: bool,
    #[serde(default)]
    synthesis_seen: bool,
    #[serde(default)]
    subagent_results: usize,
    #[serde(default)]
    edited_files: Vec<String>,
}

impl HookState {
    fn load(root: &Path, session_id: Option<&str>) -> Result<Self, String> {
        let path = state_path(root, session_id);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read agent hook state: {error}"))?;
        serde_json::from_str(&content)
            .map_err(|error| format!("failed to parse agent hook state: {error}"))
    }

    fn save(&self, root: &Path) -> Result<(), String> {
        let dir = root.join(STATE_DIR);
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create agent hook state dir: {error}"))?;
        let path = dir.join("session.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to render agent hook state: {error}"))?;
        fs::write(path, content)
            .map_err(|error| format!("failed to write agent hook state: {error}"))
    }

    fn start_turn(&mut self, turn_id: Option<&str>) {
        let Some(turn_id) = turn_id else {
            return;
        };
        if self.turn_id.as_deref() == Some(turn_id) {
            return;
        }
        self.turn_id = Some(turn_id.to_string());
        self.changed_check_seen = false;
        self.synthesis_seen = false;
        self.edited_files.clear();
    }

    fn record_edit(&mut self, files: &[String]) {
        if files.is_empty() {
            return;
        }
        let mut merged = self.edited_files.iter().cloned().collect::<BTreeSet<_>>();
        merged.extend(files.iter().cloned());
        self.edited_files = merged.into_iter().collect();
        self.changed_check_seen = false;
    }
}

fn state_path(root: &Path, _session_id: Option<&str>) -> PathBuf {
    root.join(STATE_DIR).join("session.json")
}

fn user_prompt_response(payload: &HookPayload) -> Option<Value> {
    let prompt = payload.prompt.as_deref().unwrap_or_default();
    if !looks_like_complex_harness_task(prompt) {
        return None;
    }
    Some(context(
        HookEvent::UserPromptSubmit,
        "Complex Rust task: run `rs-harness search prime`, choose axes, then use multi-pipe or read-only subagents.",
    ))
}

fn pre_tool_response(
    payload: &HookPayload,
    policy: &HookPolicy,
    state: &HookState,
) -> Option<Value> {
    let command = tool_command(payload);
    if let Some(reason) = raw_search_reason(&command, policy) {
        return Some(pre_tool_policy_response(policy, reason));
    }

    if !policy.prime_required_before_edit || state.prime_seen {
        return None;
    }
    let touched = touched_agent_files(payload, &command);
    if touched.is_empty() {
        return None;
    }
    if policy.exact_file_edit_exception && touched.len() == 1 {
        if policy.nudges() {
            return Some(context(
                HookEvent::PreToolUse,
                "Exact-file edit allowed before prime; run `rs-harness check --changed` after editing.",
            ));
        }
        return None;
    }
    Some(pre_tool_policy_response(
        policy,
        "Run deep prime before editing Rust/TS/package files: `rs-harness search prime`.",
    ))
}

fn post_tool_response(
    _root: &Path,
    payload: &HookPayload,
    policy: &HookPolicy,
    state: &mut HookState,
) -> Option<Value> {
    let command = tool_command(payload);
    if is_prime_command(&command) {
        state.prime_seen = true;
    }
    if is_ingest_command(&command) {
        state.ingest_seen = true;
    }
    if is_changed_check_command(&command) {
        state.changed_check_seen = true;
        return None;
    }

    let touched = touched_agent_files(payload, &command);
    state.record_edit(&touched);
    if touched.is_empty() || !policy.changed_check_required_after_edit || !policy.blocks() {
        return None;
    }
    Some(json!({
        "decision": "block",
        "reason": "Run `rs-harness check --changed` after editing Rust/TS/package files, then repair compact findings."
    }))
}

fn subagent_stop_response(
    payload: &HookPayload,
    policy: &HookPolicy,
    state: &mut HookState,
) -> Option<Value> {
    if payload.stop_hook_active || !policy.subagent_evidence_required {
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
    if policy.blocks() {
        Some(json!({
            "decision": "block",
            "reason": "Return one compact line: `[search-subagent] role=... evidence=... missing=... next=... risk=...`."
        }))
    } else if policy.nudges() {
        Some(json!({
            "systemMessage": "Subagent result should include `[search-subagent] ... evidence=... missing=... next=... risk=...`."
        }))
    } else {
        None
    }
}

fn stop_response(
    payload: &HookPayload,
    policy: &HookPolicy,
    state: &mut HookState,
) -> Option<Value> {
    if payload.stop_hook_active {
        return None;
    }
    let message = payload
        .last_assistant_message
        .as_deref()
        .unwrap_or_default();
    if message.contains("[search-synthesis]") {
        state.synthesis_seen = true;
    }
    if policy.changed_check_required_after_edit
        && !state.edited_files.is_empty()
        && !state.changed_check_seen
    {
        let reason = "Rust/TS/package files changed but no changed check was recorded. Run `rs-harness check --changed`.";
        return Some(stop_policy_response(policy, reason));
    }
    if policy.synthesis_required_after_edit
        && !state.edited_files.is_empty()
        && !state.synthesis_seen
    {
        let reason = "Before final response, produce `[search-synthesis] ...` with evidence, missing, next, and edit fields.";
        return Some(stop_policy_response(policy, reason));
    }
    None
}

fn stop_policy_response(policy: &HookPolicy, reason: &str) -> Value {
    if policy.blocks() {
        json!({
            "decision": "block",
            "reason": reason
        })
    } else {
        json!({
            "systemMessage": reason
        })
    }
}

fn raw_search_reason(command: &str, policy: &HookPolicy) -> Option<&'static str> {
    if command.trim().is_empty() || is_ingest_command(command) {
        return None;
    }
    if policy.raw_rg_requires_ingest && broad_raw_tool(command, "rg") {
        return Some(
            "Raw broad rg must pipe through ingest: `rg -n \"<query>\" src tests | rs-harness search ingest`.",
        );
    }
    if policy.raw_fd_requires_ingest && broad_raw_tool(command, "fd") {
        return Some(
            "Raw fd candidate streams must pipe through ingest: `fd -e rs \"<name>\" src tests | rs-harness search ingest`.",
        );
    }
    if policy.raw_ast_grep_blocked && command_has_tool(command, "ast-grep") {
        return Some(
            "Use harness pattern recipes instead of raw ast-grep: `rs-harness search pattern <recipe>`.",
        );
    }
    None
}

fn pre_tool_policy_response(policy: &HookPolicy, reason: &str) -> Value {
    if policy.blocks() {
        json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason
            }
        })
    } else {
        context(HookEvent::PreToolUse, reason)
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

fn tool_command(payload: &HookPayload) -> String {
    payload
        .tool_input
        .get("command")
        .or_else(|| payload.tool_input.get("cmd"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn touched_agent_files(payload: &HookPayload, command: &str) -> Vec<String> {
    if !is_edit_tool(payload) && !command.contains("*** ") {
        return Vec::new();
    }
    let mut files = BTreeSet::<String>::new();
    collect_paths_from_tool_input(&payload.tool_input, &mut files);
    for line in command.lines() {
        for prefix in [
            "*** Update File: ",
            "*** Add File: ",
            "*** Delete File: ",
            "*** Move to: ",
        ] {
            if let Some(path) = line.strip_prefix(prefix) {
                files.insert(path.trim().to_string());
            }
        }
    }
    files
        .into_iter()
        .filter(|path| is_agent_relevant_path(path))
        .collect()
}

fn collect_paths_from_tool_input(value: &Value, files: &mut BTreeSet<String>) {
    match value {
        Value::String(text) if is_agent_relevant_path(text) => {
            files.insert(text.to_string());
        }
        Value::Array(values) => {
            for value in values {
                collect_paths_from_tool_input(value, files);
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(key.as_str(), "path" | "file" | "filename")
                    && let Some(path) = value.as_str()
                    && is_agent_relevant_path(path)
                {
                    files.insert(path.to_string());
                }
                collect_paths_from_tool_input(value, files);
            }
        }
        _ => {}
    }
}

fn is_edit_tool(payload: &HookPayload) -> bool {
    payload
        .tool_name
        .as_deref()
        .is_some_and(|tool| matches!(tool, "apply_patch" | "Edit" | "Write"))
}

fn is_agent_relevant_path(path: &str) -> bool {
    path.ends_with(".rs")
        || path.ends_with(".ts")
        || path.ends_with(".tsx")
        || path.ends_with("Cargo.toml")
        || path.ends_with("package.json")
        || path.ends_with("tsconfig.json")
}

fn broad_raw_tool(command: &str, tool: &str) -> bool {
    if !command_has_tool(command, tool) {
        return false;
    }
    let lower = command.to_ascii_lowercase();
    lower.contains(" src")
        || lower.contains(" tests")
        || lower.contains(" .")
        || lower.contains("./")
        || lower.lines().any(|line| {
            let words = line.split_whitespace().collect::<Vec<_>>();
            words.first() == Some(&tool) && words.len() <= 3
        })
}

fn command_has_tool(command: &str, tool: &str) -> bool {
    command
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|part| part == tool)
}

fn is_prime_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("rs-harness search prime") || lower.contains("ts-harness search prime")
}

fn is_ingest_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("rs-harness search ingest") || lower.contains("ts-harness search ingest")
}

fn is_changed_check_command(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("rs-harness check --changed") || lower.contains("ts-harness check --changed")
}

fn hook_project_root(default_root: &Path, payload: &HookPayload) -> PathBuf {
    payload
        .cwd
        .as_deref()
        .and_then(find_project_root)
        .unwrap_or_else(|| default_root.to_path_buf())
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
