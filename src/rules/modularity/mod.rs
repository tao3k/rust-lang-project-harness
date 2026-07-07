//! Rust modularity rule pack.

mod catalog;
mod entrypoints;
mod pack;
mod reasoning_tree;
mod source_shape;

pub use pack::rust_modularity_rules;
pub(crate) use pack::{
    MAX_SOURCE_EFFECTIVE_LINES, MAX_SOURCE_LINES, MIN_SOURCE_IMPLEMENTATION_ITEMS,
    MIN_SOURCE_PUBLIC_ITEMS, PACK_ID, RUST_MOD_R001, RUST_MOD_R002, RUST_MOD_R003, RUST_MOD_R004,
    RUST_MOD_R005, RUST_MOD_R006, RUST_MOD_R007, RUST_MOD_R008, RUST_MOD_R009, RUST_MOD_R010,
    RUST_MOD_R011, evaluate,
};
