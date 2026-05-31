//! Rust profile bridge for the root semantic-agent-hook runtime.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

const PROFILE_REGISTRY_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-agent-hook-profile-registry";
const PROFILE_REGISTRY_SCHEMA_VERSION: &str = "1";
const HOOK_PROTOCOL_ID: &str = "agent.semantic-protocols.agent-hooks";
const HOOK_PROTOCOL_VERSION: &str = "1";
const RUST_PROVIDER_NAMESPACE: &str = "agent.semantic-protocols.languages.rust.rs-harness";
const RUST_PROFILE_PATH: &str = ".codex/semantic-agent-hook/profiles.rs-harness.json";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) enum AgentConfigScope {
    #[default]
    Project,
    CodexProfile(String),
}

impl AgentConfigScope {
    pub(crate) fn codex_profile(profile: Option<String>) -> Result<Self, String> {
        let profile = profile.unwrap_or_else(|| "rs-harness".to_string());
        validate_codex_profile_name(&profile)?;
        Ok(Self::CodexProfile(profile))
    }

    pub(crate) fn profile_name(&self) -> Option<&str> {
        match self {
            Self::Project => None,
            Self::CodexProfile(profile) => Some(profile),
        }
    }
}

pub(crate) fn install_agent_assets(
    project_root: &Path,
    client: &str,
    scope: &AgentConfigScope,
) -> Result<String, String> {
    ensure_codex_client(client)?;
    ensure_project_scope(scope)?;
    let profile_path = write_rust_agent_profile_registry(project_root)?;
    run_semantic_agent_hook(
        [
            "install".to_string(),
            "--client".to_string(),
            "codex".to_string(),
            "--profiles".to_string(),
            profile_path.display().to_string(),
            project_root.display().to_string(),
        ],
        project_root,
        "",
    )
}

pub(crate) fn print_agent_doctor(
    project_root: &Path,
    _action: &str,
    client: Option<&str>,
    scope: &AgentConfigScope,
) -> Result<(), String> {
    let Some(client) = client else {
        println!(
            "[agent-doctor] client=none profiles=false runtime=semantic-agent-hook protocol=cli-cold"
        );
        println!(
            "|note kind=client-required message=\"pass --client codex to inspect Codex semantic hook assets\""
        );
        return Ok(());
    };
    ensure_codex_client(client)?;
    ensure_project_scope(scope)?;
    let profile_path = ensure_rust_agent_profile_registry(project_root)?;
    let output = run_semantic_agent_hook(
        [
            "doctor".to_string(),
            "--client".to_string(),
            "codex".to_string(),
            "--profiles".to_string(),
            profile_path.display().to_string(),
            project_root.display().to_string(),
        ],
        project_root,
        "",
    )?;
    print!("{output}");
    Ok(())
}

pub(crate) fn ensure_rust_agent_profile_registry(project_root: &Path) -> Result<PathBuf, String> {
    let profile_path = rust_agent_profile_registry_path(project_root);
    if profile_path.exists() {
        return Ok(profile_path);
    }
    write_rust_agent_profile_registry(project_root)
}

pub(crate) fn write_rust_agent_profile_registry(project_root: &Path) -> Result<PathBuf, String> {
    let profile_path = rust_agent_profile_registry_path(project_root);
    let Some(parent) = profile_path.parent() else {
        return Err(format!(
            "profile registry path has no parent: {}",
            profile_path.display()
        ));
    };
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let profile = serde_json::to_string_pretty(&rust_agent_profile_registry())
        .map_err(|error| format!("failed to serialize Rust agent profile: {error}"))?;
    fs::write(&profile_path, format!("{profile}\n"))
        .map_err(|error| format!("failed to write {}: {error}", profile_path.display()))?;
    Ok(profile_path)
}

pub(crate) fn rust_agent_profile_registry_path(project_root: &Path) -> PathBuf {
    project_root.join(RUST_PROFILE_PATH)
}

pub(crate) fn rust_agent_profile_registry() -> Value {
    json!({
        "schemaId": PROFILE_REGISTRY_SCHEMA_ID,
        "schemaVersion": PROFILE_REGISTRY_SCHEMA_VERSION,
        "protocolId": HOOK_PROTOCOL_ID,
        "protocolVersion": HOOK_PROTOCOL_VERSION,
        "projectRoot": ".",
        "profiles": [{
            "languageId": "rust",
            "providerId": "rs-harness",
            "binary": "rs-harness",
            "namespace": RUST_PROVIDER_NAMESPACE,
            "sourceExtensions": [".rs"],
            "configFiles": ["Cargo.toml", "Cargo.lock"],
            "sourceRoots": ["src", "tests", "crates", "examples", "benches"],
            "ignoredPathPrefixes": [
                ".cache",
                ".direnv",
                ".git",
                ".idea",
                ".jj",
                ".run",
                ".vscode",
                "node_modules",
                "target",
                ".codex/harness-state",
                ".codex/rs-harness"
            ],
            "policy": {
                "blockDirectRead": true,
                "blockBroadRawSearch": true,
                "blockAgentSearchJson": true,
                "requirePrimeBeforeEdit": true
            },
            "commands": {
                "prime": {"argv": ["rs-harness", "search", "prime", "--view", "seeds", "."]},
                "owner": {"argv": ["rs-harness", "search", "owner", "{path}", "items", "--view", "seeds", "."]},
                "text": {"argv": ["rs-harness", "search", "text", "{query}", "tests", "--view", "seeds", "."]},
                "ingest": {"argv": ["rs-harness", "search", "ingest", "items", "tests", "--view", "seeds", "."], "stdinMode": "pipe-candidates"},
                "checkChanged": {"argv": ["rs-harness", "check", "--changed", "."]}
            }
        }]
    })
}

pub(crate) fn run_semantic_agent_hook<I>(args: I, cwd: &Path, stdin: &str) -> Result<String, String>
where
    I: IntoIterator<Item = String>,
{
    let mut child = Command::new("semantic-agent-hook")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "semantic-agent-hook binary is required for Codex hook install/runtime".to_string()
            } else {
                format!("failed to spawn semantic-agent-hook: {error}")
            }
        })?;
    if !stdin.is_empty() {
        use std::io::Write as _;

        child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open semantic-agent-hook stdin".to_string())?
            .write_all(stdin.as_bytes())
            .map_err(|error| format!("failed to write semantic-agent-hook stdin: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for semantic-agent-hook: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(format!("semantic-agent-hook failed: {detail}"));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("semantic-agent-hook emitted non-UTF8 stdout: {error}"))
}

fn ensure_codex_client(client: &str) -> Result<(), String> {
    if client == "codex" {
        Ok(())
    } else {
        Err(format!("unsupported agent client: {client}"))
    }
}

fn ensure_project_scope(scope: &AgentConfigScope) -> Result<(), String> {
    match scope {
        AgentConfigScope::Project => Ok(()),
        AgentConfigScope::CodexProfile(profile) => Err(format!(
            "rs-harness no longer writes Codex profile hook configs directly; run semantic-agent-hook for profile-level install: {profile}"
        )),
    }
}

fn validate_codex_profile_name(profile: &str) -> Result<(), String> {
    if !profile.is_empty()
        && profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Ok(());
    }
    Err(format!(
        "invalid Codex profile name: {profile}; expected ASCII letters, digits, '-' or '_'"
    ))
}
