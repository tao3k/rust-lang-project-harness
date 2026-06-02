//! Rendering for `ReviewPacket` outputs.

use super::model::RustReviewPacket;

/// Render a compact review-packet line.
#[must_use]
pub fn render_rust_review_packet(packet: &RustReviewPacket) -> String {
    format!(
        "review-packet changed-invariants={} changed-behavior={} missing-receipts={} stale-waivers={} determinism-observations={} proof-claims={} actions={}",
        packet.summary.changed_invariants,
        packet.summary.changed_behavior,
        packet.summary.missing_receipts,
        packet.summary.stale_waivers,
        packet.summary.determinism_observations,
        packet.summary.proof_claims,
        packet.review_actions.len()
    )
}

/// Render review packet JSON.
///
/// # Errors
///
/// Returns an error when serialization fails.
pub fn render_rust_review_packet_json(packet: &RustReviewPacket) -> Result<String, String> {
    serde_json::to_string_pretty(packet)
        .map_err(|error| format!("failed to render review packet JSON: {error}"))
}
