//! Serializable `semantic-evidence-graph` packet model for Rust provider output.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Shared `EvidenceGraph` schema id.
pub const RUST_EVIDENCE_GRAPH_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-evidence-graph";
/// Shared `EvidenceGraph` schema version.
pub const RUST_EVIDENCE_GRAPH_SCHEMA_VERSION: &str = "1";
/// Shared `EvidenceGraph` protocol id.
pub const RUST_EVIDENCE_GRAPH_PROTOCOL_ID: &str = "agent.semantic-protocols.evidence-graph";
/// Shared `EvidenceGraph` protocol version.
pub const RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION: &str = "1";

/// raw dto boundary: serialized evidence graph packet mirrors the shared schema.
/// Evidence graph artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraph {
    /// Shared schema id.
    pub schema_id: String,
    /// Shared schema version.
    pub schema_version: String,
    /// Shared protocol id.
    pub protocol_id: String,
    /// Shared protocol version.
    pub protocol_version: String,
    /// Stable graph id.
    pub graph_id: String,
    /// Producer metadata.
    pub producer: RustEvidenceGraphProducer,
    /// Project metadata.
    pub project: RustEvidenceGraphProject,
    /// Compact counters.
    pub summary: RustEvidenceGraphSummary,
    /// Evidence nodes.
    pub nodes: Vec<RustEvidenceNode>,
    /// Evidence edges.
    pub edges: Vec<RustEvidenceEdge>,
    /// Compact missing-evidence summaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<RustEvidenceGap>,
    /// Provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Evidence graph producer metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphProducer {
    pub language_id: String,
    pub provider_id: String,
    pub namespace: String,
}

/// Evidence graph project metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphProject {
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Evidence graph summary counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphSummary {
    pub nodes: usize,
    pub edges: usize,
    pub owners: usize,
    pub claims: usize,
    pub stale_items: usize,
    pub gaps: usize,
}

/// raw dto boundary: serialized evidence node mirrors the shared schema.
/// Evidence graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceNode {
    pub node_id: String,
    pub kind: RustEvidenceNodeKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RustEvidenceNodeStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<RustEvidenceLocation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Evidence graph node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceNodeKind {
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

/// Evidence graph node status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceNodeStatus {
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

/// Project-relative location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
}

/// raw dto boundary: serialized evidence edge mirrors the shared schema.
/// Evidence graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceEdge {
    pub edge_id: String,
    pub kind: RustEvidenceEdgeKind,
    pub from_node_id: String,
    pub to_node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Evidence graph edge kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceEdgeKind {
    DerivedFrom,
    RequiresEvidence,
    VerifiedBy,
    ObservedBy,
    WaivedBy,
    ReviewedBy,
    SuggestsAction,
    SupportsClaim,
}

/// Missing or incomplete evidence gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGap {
    pub gap_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}
