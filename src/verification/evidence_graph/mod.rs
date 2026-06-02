//! Evidence-graph packet construction from review packet facts.

mod build;
mod model;
mod render;

pub use build::{RustEvidenceGraphInput, build_rust_evidence_graph};
pub use model::{
    RUST_EVIDENCE_GRAPH_PROTOCOL_ID, RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION,
    RUST_EVIDENCE_GRAPH_SCHEMA_ID, RUST_EVIDENCE_GRAPH_SCHEMA_VERSION, RustEvidenceEdge,
    RustEvidenceEdgeKind, RustEvidenceGap, RustEvidenceGraph, RustEvidenceGraphProducer,
    RustEvidenceGraphProject, RustEvidenceGraphSummary, RustEvidenceLocation, RustEvidenceNode,
    RustEvidenceNodeKind, RustEvidenceNodeStatus,
};
pub use render::{render_rust_evidence_graph, render_rust_evidence_graph_json};
