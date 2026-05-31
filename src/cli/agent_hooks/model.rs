use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

pub(super) const CODEX_POLICY_PATH: &str = ".codex/harness-policy.json";
pub(super) const CODEX_STATE_DIR: &str = ".codex/harness-state";
pub(super) const RUST_CHECK: &str = "rs-harness check --changed";
pub(super) const TS_CHECK: &str = "ts-harness check --changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Profile {
    Rust,
    TypeScript,
}

impl Profile {
    pub(super) fn language_id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
        }
    }

    pub(super) fn provider_id(self) -> &'static str {
        match self {
            Self::Rust => "rs-harness",
            Self::TypeScript => "ts-harness",
        }
    }

    pub(super) fn binary(self) -> &'static str {
        self.provider_id()
    }

    pub(super) fn check_command(self) -> &'static str {
        match self {
            Self::Rust => RUST_CHECK,
            Self::TypeScript => TS_CHECK,
        }
    }

    pub(super) fn display(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TS/JS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HookEvent {
    SessionStart,
    UserPromptSubmit,
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    SubagentStart,
    SubagentStop,
    Stop,
}

impl HookEvent {
    pub(super) fn codex_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PermissionRequest => "PermissionRequest",
            Self::PostToolUse => "PostToolUse",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::Stop => "Stop",
        }
    }

    pub(super) fn semantic_name(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::UserPromptSubmit => "user-prompt",
            Self::PreToolUse => "pre-tool",
            Self::PermissionRequest => "permission-request",
            Self::PostToolUse => "post-tool",
            Self::SubagentStart => "subagent-start",
            Self::SubagentStop => "subagent-stop",
            Self::Stop => "stop",
        }
    }
}

pub(super) fn normalize_event(
    explicit: &str,
    payload_event: Option<&str>,
) -> Result<HookEvent, String> {
    let event = if explicit.is_empty() {
        payload_event.unwrap_or("")
    } else {
        explicit
    };
    match event {
        "session-start" | "SessionStart" => Ok(HookEvent::SessionStart),
        "user-prompt" | "UserPromptSubmit" => Ok(HookEvent::UserPromptSubmit),
        "pre-tool" | "PreToolUse" => Ok(HookEvent::PreToolUse),
        "permission-request" | "PermissionRequest" => Ok(HookEvent::PermissionRequest),
        "post-tool" | "PostToolUse" => Ok(HookEvent::PostToolUse),
        "subagent-start" | "SubagentStart" => Ok(HookEvent::SubagentStart),
        "subagent-stop" | "SubagentStop" => Ok(HookEvent::SubagentStop),
        "stop" | "Stop" => Ok(HookEvent::Stop),
        other => Err(format!("unknown codex hook event: {other}")),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct HookPayload {
    #[serde(default)]
    pub(super) turn_id: Option<String>,
    #[serde(default)]
    pub(super) hook_event_name: Option<String>,
    #[serde(default)]
    pub(super) cwd: Option<PathBuf>,
    #[serde(default)]
    pub(super) tool_name: Option<String>,
    #[serde(default)]
    pub(super) tool_input: Value,
    #[serde(default)]
    pub(super) prompt: Option<String>,
    #[serde(default)]
    pub(super) last_assistant_message: Option<String>,
    #[serde(default)]
    pub(super) stop_hook_active: bool,
}

pub(super) fn parse_hook_payload(input: &str) -> Result<HookPayload, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        serde_json::from_value(json!({}))
    } else {
        serde_json::from_str(trimmed)
    }
    .map_err(|error| format!("failed to parse codex hook JSON: {error}"))
}

pub(super) fn hook_project_root(default_root: &Path, payload: &HookPayload) -> PathBuf {
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
        if current.join("Cargo.toml").exists() || current.join("package.json").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub(super) fn rust_roots() -> &'static [&'static str] {
    &["src", "tests", "benches", "examples", "crates"]
}

pub(super) fn ts_roots() -> &'static [&'static str] {
    &["src", "tests", "apps", "packages"]
}

pub(super) fn rust_config_files() -> &'static [&'static str] {
    &[
        "cargo.toml",
        "cargo.lock",
        "build.rs",
        "rust-toolchain.toml",
        "rustfmt.toml",
        "clippy.toml",
        ".cargo/config.toml",
    ]
}

pub(super) fn ts_extensions() -> &'static [&'static str] {
    &[".ts", ".tsx", ".js", ".jsx", ".mts", ".cts", ".mjs", ".cjs"]
}

pub(super) fn ts_config_files() -> &'static [&'static str] {
    &[
        "package.json",
        "tsconfig.json",
        "tsconfig.base.json",
        "jsconfig.json",
        "vite.config.ts",
        "rspack.config.ts",
        "webpack.config.js",
        "next.config.js",
        "eslint.config.js",
        "vitest.config.ts",
        "jest.config.ts",
        "jest.config.js",
        "tsup.config.ts",
        "rollup.config.js",
        "babel.config.js",
    ]
}
