//! Evidence-graph packet construction from review packet facts.

mod analysis;
mod build;
mod model;
mod render;

pub use analysis::{
    RUST_EVIDENCE_GRAPH_ANALYSIS_PACKET_KIND, RUST_EVIDENCE_GRAPH_ANALYSIS_PROFILE,
    RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_ID,
    RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_VERSION, RustEvidenceGraphAnalysisGraph,
    RustEvidenceGraphAnalysisInput, RustEvidenceGraphAnalysisPacketKind,
    RustEvidenceGraphAnalysisProducer, RustEvidenceGraphAnalysisRequest,
    RustEvidenceGraphAnalysisSummary, build_rust_evidence_graph_analysis_request,
    render_rust_evidence_graph_analysis_request, render_rust_evidence_graph_analysis_request_json,
};
pub use build::{RustEvidenceGraphInput, build_rust_evidence_graph};
pub use model::{
    RUST_EVIDENCE_GRAPH_PROTOCOL_ID, RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION,
    RUST_EVIDENCE_GRAPH_SCHEMA_ID, RUST_EVIDENCE_GRAPH_SCHEMA_VERSION, RustEvidenceEdge,
    RustEvidenceEdgeKind, RustEvidenceGap, RustEvidenceGraph, RustEvidenceGraphProducer,
    RustEvidenceGraphProject, RustEvidenceGraphSummary, RustEvidenceLocation, RustEvidenceNode,
    RustEvidenceNodeKind, RustEvidenceNodeStatus,
};
pub use render::{render_rust_evidence_graph, render_rust_evidence_graph_json};
