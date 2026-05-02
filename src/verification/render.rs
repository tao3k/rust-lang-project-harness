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
    let mut rendered = groups
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
        .join("\n");
    let report_obligations = render_report_obligations(plan);
    if rendered.is_empty() {
        report_obligations
    } else if report_obligations.is_empty() {
        rendered
    } else {
        rendered.push('\n');
        rendered.push_str(&report_obligations);
        rendered
    }
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

/// Render configured skill descriptors as compact reasoning-tree nodes.
///
/// Default verification output only references these contracts. Agents can call
/// this renderer when they need to expand a `contract_ref` into an execution
/// standard.
#[must_use]
pub fn render_rust_verification_skill_contracts(plan: &RustVerificationPlan) -> String {
    plan.skill_descriptors
        .iter()
        .map(render_skill_descriptor)
        .collect::<Vec<_>>()
        .join("\n")
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
    let _ = write!(
        rendered,
        "   |{kind}: {} phase={} fingerprint={}",
        task.state.as_str(),
        task.phase.as_str(),
        task.fingerprint
    );
    if let Some(binding) = &task.skill_binding {
        let _ = write!(rendered, " skill={}", binding.compact_label());
    }
    if let Some(contract_ref) = &task.skill_contract_ref {
        let _ = write!(rendered, " contract_ref={contract_ref}");
    }
    let _ = writeln!(rendered);
    if let Some(line) = task.line {
        let _ = writeln!(rendered, "   |line: {kind}={line}");
    }
    if task.skill_binding.is_some() {
        render_task_resolution(task, rendered, kind);
        return;
    }
    let _ = writeln!(rendered, "   |why: {kind}={}", task.reason);
    render_task_resolution(task, rendered, kind);
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

fn render_task_resolution(task: &RustVerificationTask, rendered: &mut String, kind: &str) {
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
}

fn render_report_obligations(plan: &RustVerificationPlan) -> String {
    if plan.report_obligations.is_empty() {
        return String::new();
    }

    let mut rendered = String::from("[verify-report]\n");
    for obligation in &plan.report_obligations {
        let kinds = obligation
            .task_kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(
            rendered,
            "   |required: {} renderer={} artifact={} tasks={} kinds={}",
            obligation.key,
            obligation.renderer,
            obligation.suggested_artifact_name,
            obligation.task_count(),
            kinds
        );
    }
    rendered.trim_end().to_string()
}

fn render_skill_descriptor(descriptor: &super::RustVerificationSkillDescriptor) -> String {
    let mut rendered = format!("[skill-contract] {}\n", descriptor.compact_label());
    if !descriptor.tool.is_empty() {
        let _ = writeln!(rendered, "   |tool: {}", descriptor.tool);
    }
    if !descriptor.command.is_empty() {
        let _ = writeln!(rendered, "   |run: {}", descriptor.command);
    }
    if !descriptor.standard.is_empty() {
        let _ = writeln!(rendered, "   |standard: {}", descriptor.standard);
    }
    if !descriptor.required_inputs.is_empty() {
        let _ = writeln!(
            rendered,
            "   |inputs: {}",
            descriptor.required_inputs.join(",")
        );
    }
    if !descriptor.pass_criteria.is_empty() {
        let _ = writeln!(rendered, "   |pass: {}", descriptor.pass_criteria.join(","));
    }
    if !descriptor.receipt_fields.is_empty() {
        let _ = writeln!(
            rendered,
            "   |receipt: {}",
            descriptor.receipt_fields.join(",")
        );
    }
    rendered
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
