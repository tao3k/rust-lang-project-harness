use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;

use serde_json::{Value, json};

use super::super::agent_assets::{ensure_rust_agent_profile_registry, run_semantic_agent_hook};

pub(crate) fn run_agent_hook(project_root: &Path, client: &str, event: &str) -> Result<(), String> {
    ensure_codex_client(client)?;
    let profile_path = ensure_rust_agent_profile_registry(project_root)?;
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read agent hook stdin: {error}"))?;
    let output = run_semantic_agent_hook(
        [
            "hook".to_string(),
            "--client".to_string(),
            "codex".to_string(),
            event.to_string(),
            "--profiles".to_string(),
            profile_path.display().to_string(),
        ],
        project_root,
        &input,
    )?;
    print!("{output}");
    Ok(())
}

pub(crate) fn run_agent_guard(
    project_root: &Path,
    client: &str,
    command_args: &[OsString],
    json_output: bool,
) -> Result<bool, String> {
    ensure_codex_client(client)?;
    let command = guard_command(command_args)?;
    let profile_path = ensure_rust_agent_profile_registry(project_root)?;
    let payload = json!({
        "hook_event_name": "PreToolUse",
        "cwd": project_root.display().to_string(),
        "tool_name": "functions.exec_command",
        "tool_input": {
            "cmd": command
        }
    })
    .to_string();
    let output = run_semantic_agent_hook(
        [
            "hook".to_string(),
            "--client".to_string(),
            "codex".to_string(),
            "pre-tool".to_string(),
            "--profiles".to_string(),
            profile_path.display().to_string(),
            "--emit".to_string(),
            "decision".to_string(),
        ],
        project_root,
        &payload,
    )?;
    if json_output {
        print!("{output}");
    }
    let decision = serde_json::from_str::<Value>(&output)
        .map_err(|error| format!("semantic-agent-hook emitted invalid decision JSON: {error}"))?;
    if decision
        .get("decision")
        .and_then(Value::as_str)
        .is_some_and(|decision| decision == "allow")
    {
        return Ok(true);
    }
    if !json_output && let Some(message) = decision.get("message").and_then(Value::as_str) {
        eprintln!("{message}");
    }
    Ok(false)
}

pub(crate) fn print_agent_guide(_project_root: &Path, client: &str) -> Result<(), String> {
    ensure_codex_client(client)?;
    println!(
        "[agent-guide] runtime=semantic-agent-hook language=rust provider=rs-harness\n\
         |flow prime->owner|deps|symbol|tests pipe=text:tests ingest=stdin\n\
         |cmd prime=rs-harness search prime --view seeds .\n\
         |cmd owner=rs-harness search owner <path> items --view seeds .\n\
         |cmd ingest=rg -n '<query>' src tests | rs-harness search ingest items tests --view seeds ."
    );
    Ok(())
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

fn ensure_codex_client(client: &str) -> Result<(), String> {
    if client == "codex" {
        Ok(())
    } else {
        Err(format!("unsupported agent hook client: {client}"))
    }
}
