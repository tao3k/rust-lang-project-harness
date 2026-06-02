//! Render `semantic-assurance-case` packets for agent compact text and JSON.

use super::model::RustAssuranceCaseSet;

/// Render a compact assurance-case line.
#[must_use]
pub fn render_rust_assurance_case_set(case_set: &RustAssuranceCaseSet) -> String {
    format!(
        "assurance-case cases={} claims={} supported={} gaps={} stale-items={}",
        case_set.summary.cases,
        case_set.summary.claims,
        case_set.summary.supported_claims,
        case_set.summary.open_gaps,
        case_set.summary.stale_items
    )
}

/// Render assurance case JSON.
pub fn render_rust_assurance_case_set_json(
    case_set: &RustAssuranceCaseSet,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(case_set)
}
