//! Rust native projection into tree-sitter-compatible query captures.

mod calls;
mod core;

pub(super) use core::{
    SUPPORTED_TREE_SITTER_QUERY_NODES, SyntaxQueryPredicate, SyntaxQueryPredicateOp,
    SyntaxQueryPredicateValue, project_tree_sitter_query,
};
