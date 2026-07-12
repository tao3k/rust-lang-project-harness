mod core;
mod predicate;

pub(super) use core::project_native_tree_sitter_query;
pub(super) use predicate::{
    SyntaxQueryPredicate, SyntaxQueryPredicateOp, SyntaxQueryPredicateValue,
};
