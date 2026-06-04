use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

pub(in crate::cli) fn render_search_graph_packet(
    packet: &Value,
    seed_limit: Option<usize>,
) -> Result<String, String> {
    let binary = std::env::var_os("SEMANTIC_AGENT_PROTOCOL_BIN").unwrap_or_else(|| "asp".into());
    let mut command = Command::new(binary);
    command.args(["graph", "render", "--packet", "-", "--view", "seeds"]);
    if let Some(limit) = seed_limit {
        command.args(["--seeds", &limit.to_string()]);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!("failed to spawn semantic-agent-protocol graph render: {error}")
        })?;

    let encoded = serde_json::to_vec(packet)
        .map_err(|error| format!("failed to encode graph packet: {error}"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open semantic-agent-protocol stdin".to_string())?;
        stdin
            .write_all(&encoded)
            .map_err(|error| format!("failed to write graph packet: {error}"))?;
        stdin
            .write_all(b"\n")
            .map_err(|error| format!("failed to finish graph packet: {error}"))?;
    }

    let output = child.wait_with_output().map_err(|error| {
        format!("failed to wait for semantic-agent-protocol graph render: {error}")
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            output.status.to_string()
        } else {
            stderr
        };
        return Err(format!(
            "semantic-agent-protocol graph render failed: {detail}"
        ));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        format!("semantic-agent-protocol graph render emitted non-UTF-8 output: {error}")
    })
}
