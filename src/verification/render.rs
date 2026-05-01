//! Compact rendering for verification tasks.

use std::fmt::Write;
use std::path::Path;

use super::{RustVerificationPlan, RustVerificationTask};

/// Render active verification tasks for agent consumption.
///
/// Satisfied and waived tasks are intentionally omitted so a matching receipt
/// or waiver removes the reminder from the compact agent channel.
#[must_use]
pub fn render_rust_verification_plan(plan: &RustVerificationPlan) -> String {
    let display_root = if plan.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(plan.project_root.as_path())
    };
    plan.active_tasks()
        .into_iter()
        .map(|task| render_task(task, display_root.unwrap_or(&task.package_root)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a structured JSON verification plan for tool consumers.
///
/// # Errors
///
/// Returns a serialization error if the plan cannot be encoded as JSON.
pub fn render_rust_verification_plan_json(
    plan: &RustVerificationPlan,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(plan)
}

fn render_task(task: &RustVerificationTask, display_root: &Path) -> String {
    let mut rendered = format!(
        "[verify:{}] {} {}\n",
        task.kind.as_str(),
        task.state.as_str(),
        display_project_path(display_root, &task.owner_path)
    );
    if let Some(line) = task.line {
        let _ = writeln!(rendered, "   |line: {line}");
    }
    if !task.owner_namespace.is_empty() {
        let _ = writeln!(rendered, "   |owner: {}", task.owner_namespace.join("/"));
    }
    let _ = writeln!(rendered, "   |phase: {}", task.phase.as_str());
    let _ = writeln!(rendered, "   |why: {}", task.reason);
    if let Some(summary) = &task.receipt_summary {
        let _ = writeln!(rendered, "   |receipt: {summary}");
    }
    for note in &task.resolution_notes {
        let _ = writeln!(rendered, "   |resolution: {}={}", note.label, note.detail);
    }
    for fact in &task.evidence {
        let _ = writeln!(rendered, "   |fact: {}={}", fact.label, fact.value);
    }
    let _ = writeln!(rendered, "   |contract: {}", task.required_receipt);
    let _ = writeln!(rendered, "   |fingerprint: {}", task.fingerprint);
    rendered
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
