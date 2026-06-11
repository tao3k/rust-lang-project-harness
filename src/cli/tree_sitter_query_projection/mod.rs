//! Rust native projection into tree-sitter-compatible query captures.

mod calls;
mod capture;
mod core;
mod predicate;
mod prefilter;

pub(super) use core::{SUPPORTED_TREE_SITTER_QUERY_NODES, project_tree_sitter_query};
pub(super) use predicate::{
    SyntaxQueryPredicate, SyntaxQueryPredicateOp, SyntaxQueryPredicateValue,
};

#[cfg(test)]
#[path = "../../../tests/unit/cli/tree_sitter_query_projection_prefilter.rs"]
mod prefilter_tests;
