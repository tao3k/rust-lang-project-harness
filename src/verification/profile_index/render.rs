//! Compact agent renderer for parser-derived verification profile candidates.

use std::fmt::Write;
use std::path::Path;

use crate::verification::profile::{responsibility_labels, task_kind_labels};

use super::model::{RustVerificationProfileCandidate, RustVerificationProfileIndex};

/// Render active responsibility-profile candidates for agents.
#[must_use]
pub fn render_rust_verification_profile_index(index: &RustVerificationProfileIndex) -> String {
    let display_root = if index.project_root.as_os_str().is_empty() {
        None
    } else {
        Some(index.project_root.as_path())
    };
    let candidates = index
        .active_candidates()
        .into_iter()
        .map(|candidate| render_profile_candidate(candidate, display_root))
        .collect::<Vec<_>>()
        .join("\n");
    if !index.needs_profile_configuration() {
        return candidates;
    }
    let reminder = render_profile_configuration_reminder(index);
    if candidates.is_empty() {
        reminder
    } else {
        format!("{reminder}\n{candidates}")
    }
}

/// Render responsibility-profile candidates as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the index cannot be encoded as JSON.
pub fn render_rust_verification_profile_index_json(
    index: &RustVerificationProfileIndex,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(index)
}

fn render_profile_candidate(
    candidate: &RustVerificationProfileCandidate,
    display_root: Option<&Path>,
) -> String {
    let display_root = display_root.unwrap_or(&candidate.package_root);
    let mut rendered = format!(
        "[verify-profile] {}\n",
        display_project_path(display_root, &candidate.owner_path)
    );
    if !candidate.owner_namespace.is_empty() {
        let _ = writeln!(
            rendered,
            "   |owner: {}",
            candidate.owner_namespace.join("/")
        );
    }
    let _ = writeln!(rendered, "   |state: {}", candidate.state.as_str());
    if !candidate.configured_responsibilities.is_empty() {
        let _ = writeln!(
            rendered,
            "   |configured: {}",
            responsibility_labels(&candidate.configured_responsibilities)
        );
    }
    let _ = writeln!(
        rendered,
        "   |suggest: {}",
        responsibility_labels(&candidate.suggested_responsibilities)
    );
    let _ = writeln!(
        rendered,
        "   |tasks: {}",
        task_kind_labels(&candidate.suggested_task_kinds)
    );
    for fact in candidate
        .evidence
        .iter()
        .filter(|fact| compact_fact(&fact.label))
    {
        let _ = writeln!(rendered, "   |fact: {}={}", fact.label, fact.value);
    }
    rendered
}

fn render_profile_configuration_reminder(index: &RustVerificationProfileIndex) -> String {
    format!(
        "[verify-profile] profile_hints\n   |state: missing_profile_config\n   |action: configure RustVerificationProfileHint entries\n   |candidates: {}",
        index.active_candidates().len()
    )
}

fn compact_fact(label: &str) -> bool {
    label != "dependency_roots"
}

fn display_project_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
