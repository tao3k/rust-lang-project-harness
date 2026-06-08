//! Collection field semantic graph facts projected from Rust syntax.

mod emit;
mod facts;
mod field_extract;
mod owner_scan;

pub(super) use emit::emit_collection_field_graph_facts;
pub(super) use owner_scan::semantic_fact_owners;
