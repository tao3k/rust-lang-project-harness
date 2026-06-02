//! Render `semantic-evidence-graph` packets for agent compact text and JSON.

use super::model::RustEvidenceGraph;

/// Render a compact evidence-graph line.
#[must_use]
pub fn render_rust_evidence_graph(graph: &RustEvidenceGraph) -> String {
    format!(
        "evidence-graph nodes={} edges={} owners={} claims={} stale-items={} gaps={}",
        graph.summary.nodes,
        graph.summary.edges,
        graph.summary.owners,
        graph.summary.claims,
        graph.summary.stale_items,
        graph.summary.gaps
    )
}

/// Render evidence graph JSON.
pub fn render_rust_evidence_graph_json(
    graph: &RustEvidenceGraph,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(graph)
}
