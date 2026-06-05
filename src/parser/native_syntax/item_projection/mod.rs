//! Parser-owned item projection facts for compact source views.

mod api;
mod labels;
mod node_walk;
mod token_compact;

pub(crate) use api::RustItemProjectionNodeSyntax;
pub(super) use api::item_projection_nodes;
