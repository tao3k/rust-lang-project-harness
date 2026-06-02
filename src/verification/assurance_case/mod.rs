//! Assurance-case packet construction from reviewer evidence graph facts.

mod build;
mod model;
mod render;

pub use build::{RustAssuranceCaseInput, build_rust_assurance_case_set};
pub use model::{
    RUST_ASSURANCE_CASE_PROTOCOL_ID, RUST_ASSURANCE_CASE_PROTOCOL_VERSION,
    RUST_ASSURANCE_CASE_SCHEMA_ID, RUST_ASSURANCE_CASE_SCHEMA_VERSION, RustAssuranceActionRef,
    RustAssuranceCase, RustAssuranceCaseSet, RustAssuranceCaseSetProducer,
    RustAssuranceCaseSetProject, RustAssuranceCaseStatus, RustAssuranceCaseSummary,
    RustAssuranceClaim, RustAssuranceClaimKind, RustAssuranceGap, RustAssuranceNodeKind,
    RustAssuranceNodeRef, RustAssuranceNodeStatus,
};
pub use render::{render_rust_assurance_case_set, render_rust_assurance_case_set_json};
