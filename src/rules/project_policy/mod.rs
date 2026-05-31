//! Project-level Rust test policy rules.

mod build_gate;
mod catalog;
mod config;
mod pack;
mod source_scope;
mod source_tests;
mod support;
mod test_bloat;
mod test_layout;
mod test_targets;
mod verification_integration;

pub use pack::rust_project_policy_rules;
pub(crate) use pack::{
    MAX_INTEGRATION_TEST_EFFECTIVE_LINES, MAX_UNIT_TEST_EFFECTIVE_LINES,
    MIN_INTEGRATION_TEST_FUNCTIONS, MIN_UNIT_TEST_FUNCTIONS, PACK_ID, RUST_PROJ_R001,
    RUST_PROJ_R002, RUST_PROJ_R003, RUST_PROJ_R004, RUST_PROJ_R005, RUST_PROJ_R006, RUST_PROJ_R007,
    RUST_PROJ_R008, RUST_PROJ_R009, RUST_PROJ_R010, RUST_PROJ_R011, RUST_PROJ_R012, RUST_PROJ_R013,
    RUST_PROJ_R014, RUST_PROJ_R015, RUST_PROJ_R016, evaluate,
};
