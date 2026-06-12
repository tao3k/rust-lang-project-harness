//! Agent-facing downstream verification gate guidance.

/// Markdown guide for downstream crates that consume this harness as a library.
pub const RUST_DOWNSTREAM_VERIFICATION_GATE_GUIDE_MD: &str =
    include_str!("../docs/downstream-verification-gate.md");

/// Return the downstream verification gate guide.
#[must_use]
pub fn rust_downstream_verification_gate_guide_markdown() -> &'static str {
    RUST_DOWNSTREAM_VERIFICATION_GATE_GUIDE_MD
}
