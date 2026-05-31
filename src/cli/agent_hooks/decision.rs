use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::model::{HookEvent, HookPayload, Profile};

#[derive(Debug, Clone, Copy)]
pub(super) enum HookDecision {
    Context,
    Deny,
    Block,
}

impl HookDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Deny => "deny",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ReasonKind {
    DirectSourceRead,
    BulkSourceDump,
    RawBroadSearch,
    EditBeforePrime,
    ChangedCheckRequired,
    SubagentReceiptRequired,
    Policy,
}

impl ReasonKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DirectSourceRead => "direct-source-read",
            Self::BulkSourceDump => "bulk-source-dump",
            Self::RawBroadSearch => "raw-broad-search",
            Self::EditBeforePrime => "edit-before-prime",
            Self::ChangedCheckRequired => "changed-check-required",
            Self::SubagentReceiptRequired => "subagent-receipt-required",
            Self::Policy => "policy",
        }
    }
}

pub(super) fn direct_source_read(
    event: HookEvent,
    payload: &HookPayload,
    command: &str,
    path: &str,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Deny,
        ReasonKind::DirectSourceRead,
        [Profile::Rust].into_iter().collect(),
        payload,
        command,
        &[path],
        vec![rust_owner_route(path), rust_tests_route(path)],
        message,
    )
}

pub(super) fn bulk_source_dump(
    event: HookEvent,
    payload: &HookPayload,
    command: &str,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Deny,
        ReasonKind::BulkSourceDump,
        [Profile::Rust].into_iter().collect(),
        payload,
        command,
        &[],
        vec![rust_ingest_route()],
        message,
    )
}

pub(super) fn raw_broad_search(
    event: HookEvent,
    payload: &HookPayload,
    command: &str,
    profiles: &BTreeSet<Profile>,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Deny,
        ReasonKind::RawBroadSearch,
        profiles.clone(),
        payload,
        command,
        &[],
        ingest_routes(profiles),
        message,
    )
}

pub(super) fn edit_before_prime(
    event: HookEvent,
    payload: &HookPayload,
    command: &str,
    profiles: &BTreeSet<Profile>,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Deny,
        ReasonKind::EditBeforePrime,
        profiles.clone(),
        payload,
        command,
        &[],
        prime_routes(profiles),
        message,
    )
}

pub(super) fn exact_file_edit_context(
    event: HookEvent,
    payload: &HookPayload,
    command: &str,
    profiles: &BTreeSet<Profile>,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Context,
        ReasonKind::Policy,
        profiles.clone(),
        payload,
        command,
        &[],
        changed_check_routes(profiles),
        message,
    )
}

pub(super) fn changed_check_required(
    event: HookEvent,
    payload: &HookPayload,
    profiles: &[Profile],
    message: &str,
) -> Value {
    let profile_set = profiles.iter().copied().collect::<BTreeSet<_>>();
    packet(
        event,
        HookDecision::Block,
        ReasonKind::ChangedCheckRequired,
        profile_set.clone(),
        payload,
        "",
        &[],
        changed_check_routes(&profile_set),
        message,
    )
}

pub(super) fn subagent_receipt_required(
    event: HookEvent,
    payload: &HookPayload,
    message: &str,
) -> Value {
    packet(
        event,
        HookDecision::Block,
        ReasonKind::SubagentReceiptRequired,
        BTreeSet::new(),
        payload,
        "",
        &[],
        Vec::new(),
        message,
    )
}

fn packet(
    event: HookEvent,
    decision: HookDecision,
    reason_kind: ReasonKind,
    profiles: BTreeSet<Profile>,
    payload: &HookPayload,
    command: &str,
    paths: &[&str],
    routes: Vec<Value>,
    message: &str,
) -> Value {
    json!({
        "schemaId": "agent.semantic-protocols.agent-hook-decision",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.agent-hooks",
        "protocolVersion": "1",
        "platform": "codex",
        "event": event.semantic_name(),
        "decision": decision.as_str(),
        "reasonKind": reason_kind.as_str(),
        "languageIds": profiles
            .iter()
            .map(|profile| profile.language_id())
            .collect::<Vec<_>>(),
        "subject": subject(payload, command, paths),
        "routes": routes,
        "message": message
    })
}

fn subject(payload: &HookPayload, command: &str, paths: &[&str]) -> Value {
    let mut fields = Map::new();
    if let Some(tool_name) = payload
        .tool_name
        .as_deref()
        .filter(|tool_name| !tool_name.is_empty())
    {
        fields.insert("toolName".to_string(), json!(tool_name));
    }
    if !command.is_empty() {
        fields.insert("command".to_string(), json!(command));
    }
    if !paths.is_empty() {
        fields.insert("paths".to_string(), json!(paths));
    }
    if let Some(prompt) = payload
        .prompt
        .as_deref()
        .filter(|prompt| !prompt.is_empty())
    {
        fields.insert("prompt".to_string(), json!(prompt));
    }
    Value::Object(fields)
}

fn ingest_routes(profiles: &BTreeSet<Profile>) -> Vec<Value> {
    profiles
        .iter()
        .map(|profile| match profile {
            Profile::Rust => rust_ingest_route(),
            Profile::TypeScript => ts_ingest_route(),
        })
        .collect()
}

fn prime_routes(profiles: &BTreeSet<Profile>) -> Vec<Value> {
    profiles
        .iter()
        .map(|profile| match profile {
            Profile::Rust => route(
                *profile,
                "prime",
                vec![
                    "rs-harness",
                    "search",
                    "prime",
                    "--view",
                    "seeds",
                    "--seeds",
                    "8",
                    ".",
                ],
                None,
            ),
            Profile::TypeScript => route(
                *profile,
                "prime",
                vec!["ts-harness", "search", "prime", "--view", "seeds", "."],
                None,
            ),
        })
        .collect()
}

fn changed_check_routes(profiles: &BTreeSet<Profile>) -> Vec<Value> {
    profiles
        .iter()
        .map(|profile| match profile {
            Profile::Rust => route(
                *profile,
                "check-changed",
                vec!["rs-harness", "check", "--changed", "."],
                None,
            ),
            Profile::TypeScript => route(
                *profile,
                "check-changed",
                vec!["ts-harness", "check", "--changed", "."],
                None,
            ),
        })
        .collect()
}

fn rust_owner_route(path: &str) -> Value {
    route(
        Profile::Rust,
        "owner",
        vec![
            "rs-harness",
            "search",
            "owner",
            path,
            "items",
            "--trace",
            "--view",
            "seeds",
            "--seeds",
            "8",
            ".",
        ],
        None,
    )
}

fn rust_tests_route(path: &str) -> Value {
    route(
        Profile::Rust,
        "tests",
        vec![
            "rs-harness",
            "search",
            "tests",
            path,
            "--view",
            "seeds",
            "--seeds",
            "4",
            ".",
        ],
        None,
    )
}

fn rust_ingest_route() -> Value {
    route(
        Profile::Rust,
        "ingest",
        vec![
            "rs-harness",
            "search",
            "ingest",
            "items",
            "tests",
            "--view",
            "seeds",
            "--seeds",
            "8",
            ".",
        ],
        Some("pipe-candidates"),
    )
}

fn ts_ingest_route() -> Value {
    route(
        Profile::TypeScript,
        "ingest",
        vec!["ts-harness", "search", "ingest", "--view", "seeds", "."],
        Some("pipe-candidates"),
    )
}

fn route(profile: Profile, kind: &str, argv: Vec<&str>, stdin_mode: Option<&'static str>) -> Value {
    let mut fields = Map::from_iter([
        ("languageId".to_string(), json!(profile.language_id())),
        ("providerId".to_string(), json!(profile.provider_id())),
        ("binary".to_string(), json!(profile.binary())),
        ("kind".to_string(), json!(kind)),
        ("argv".to_string(), json!(argv)),
        ("scope".to_string(), json!(".")),
    ]);
    if let Some(stdin_mode) = stdin_mode {
        fields.insert("stdinMode".to_string(), json!(stdin_mode));
    }
    Value::Object(fields)
}
