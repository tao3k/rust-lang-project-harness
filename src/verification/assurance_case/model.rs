//! Serializable `semantic-assurance-case` packet model for Rust provider output.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Shared `AssuranceCase` schema id.
pub const RUST_ASSURANCE_CASE_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-assurance-case";
/// Shared `AssuranceCase` schema version.
pub const RUST_ASSURANCE_CASE_SCHEMA_VERSION: &str = "1";
/// Shared `AssuranceCase` protocol id.
pub const RUST_ASSURANCE_CASE_PROTOCOL_ID: &str = "agent.semantic-protocols.assurance-case";
/// Shared `AssuranceCase` protocol version.
pub const RUST_ASSURANCE_CASE_PROTOCOL_VERSION: &str = "1";

/// raw dto boundary: serialized assurance case packet mirrors the shared schema.
/// Assurance case set artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceCaseSet {
    pub schema_id: String,
    pub schema_version: String,
    pub protocol_id: String,
    pub protocol_version: String,
    pub case_set_id: String,
    pub producer: RustAssuranceCaseSetProducer,
    pub project: RustAssuranceCaseSetProject,
    pub summary: RustAssuranceCaseSummary,
    pub cases: Vec<RustAssuranceCase>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Assurance case producer metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceCaseSetProducer {
    pub language_id: String,
    pub provider_id: String,
    pub namespace: String,
}

/// Assurance case project metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceCaseSetProject {
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Assurance case summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceCaseSummary {
    pub cases: usize,
    pub claims: usize,
    pub supported_claims: usize,
    pub open_gaps: usize,
    pub stale_items: usize,
}

/// raw dto boundary: serialized assurance case item mirrors the shared schema.
/// One assurance case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceCase {
    pub case_id: String,
    pub claim: RustAssuranceClaim,
    pub status: RustAssuranceCaseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_by: Vec<RustAssuranceNodeRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_by: Vec<RustAssuranceNodeRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_by: Vec<RustAssuranceNodeRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waived_by: Vec<RustAssuranceNodeRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<RustAssuranceActionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<RustAssuranceGap>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Assurance case status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustAssuranceCaseStatus {
    Supported,
    NeedsReview,
    Blocked,
    Unknown,
}

/// Assurance claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceClaim {
    pub claim_id: String,
    pub kind: RustAssuranceClaimKind,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Assurance claim kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustAssuranceClaimKind {
    Invariant,
    Proof,
    Review,
    Owner,
    Behavior,
    Determinism,
    Custom,
}

/// Assurance node reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceNodeRef {
    pub node_id: String,
    pub kind: RustAssuranceNodeKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RustAssuranceNodeStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Assurance graph node kind vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustAssuranceNodeKind {
    Owner,
    InvariantCandidate,
    VerificationReceipt,
    BehaviorSnapshot,
    DeterminismReadiness,
    FormalProofPilot,
    ReviewPacket,
    Waiver,
    ReviewAction,
}

/// Assurance graph node status vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustAssuranceNodeStatus {
    Current,
    Changed,
    Missing,
    Stale,
    Expired,
    Ready,
    NeedsInjection,
    Blocked,
    Unknown,
    Proved,
    ProvedBounded,
    Failed,
    Skipped,
    Error,
}

/// Assurance action reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceActionRef {
    pub node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// raw dto boundary: serialized assurance gap mirrors the shared schema.
/// Assurance gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustAssuranceGap {
    pub gap_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_gap_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}
