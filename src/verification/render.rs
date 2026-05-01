//! Compact rendering for verification tasks.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

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
    let mut groups = BTreeMap::<VerificationOwnerKey, Vec<&RustVerificationTask>>::new();
    for task in plan.active_tasks() {
        groups
            .entry(VerificationOwnerKey::from_task(task))
            .or_default()
            .push(task);
    }
    groups
        .into_iter()
        .map(|(key, mut tasks)| {
            tasks.sort_by(|left, right| {
                left.kind
                    .cmp(&right.kind)
                    .then_with(|| left.fingerprint.cmp(&right.fingerprint))
            });
            render_owner_group(&key, &tasks, display_root.unwrap_or(&key.package_root))
        })
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VerificationOwnerKey {
    package_root: PathBuf,
    owner_path: PathBuf,
    owner_namespace: Vec<String>,
}

impl VerificationOwnerKey {
    fn from_task(task: &RustVerificationTask) -> Self {
        Self {
            package_root: task.package_root.clone(),
            owner_path: task.owner_path.clone(),
            owner_namespace: task.owner_namespace.clone(),
        }
    }
}

fn render_owner_group(
    key: &VerificationOwnerKey,
    tasks: &[&RustVerificationTask],
    display_root: &Path,
) -> String {
    let mut rendered = format!(
        "[verify] {}\n",
        display_project_path(display_root, &key.owner_path)
    );
    if !key.owner_namespace.is_empty() {
        let _ = writeln!(rendered, "   |owner: {}", key.owner_namespace.join("/"));
    }
    for task in tasks {
        render_task(task, &mut rendered);
    }
    rendered
}

fn render_task(task: &RustVerificationTask, rendered: &mut String) {
    let kind = task.kind.as_str();
    let _ = writeln!(
        rendered,
        "   |{kind}: {} phase={} fingerprint={}",
        task.state.as_str(),
        task.phase.as_str(),
        task.fingerprint
    );
    if let Some(line) = task.line {
        let _ = writeln!(rendered, "   |line: {kind}={line}");
    }
    let _ = writeln!(rendered, "   |why: {kind}={}", task.reason);
    if let Some(summary) = &task.receipt_summary {
        let _ = writeln!(rendered, "   |receipt: {kind}={summary}");
    }
    for note in &task.resolution_notes {
        let _ = writeln!(
            rendered,
            "   |resolution: {kind}.{}={}",
            note.label, note.detail
        );
    }
    if !task.required_evidence.is_empty() {
        let required = task
            .required_evidence
            .iter()
            .map(|requirement| requirement.key.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(rendered, "   |requires: {kind}={required}");
    }
    for fact in &task.evidence {
        let _ = writeln!(rendered, "   |fact: {kind}.{}={}", fact.label, fact.value);
    }
    let _ = writeln!(rendered, "   |contract: {kind}={}", task.required_receipt);
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
