//! Command-line execution for the Rust project harness binary.

mod agent_registry;
mod ast_patch;
mod behavior_snapshot;
mod determinism_readiness;
mod dev_command_log;
mod evidence_graph;
mod execution_receipt;
mod flow_lite_query;
mod formal_proof_pilot;
mod query;
mod query_options;
mod query_source;
mod query_window;
mod review_packet;
mod runner;
mod runner_support;
#[cfg(feature = "search")]
mod search_output;
#[cfg(feature = "search")]
mod search_plan;
#[cfg(feature = "search")]
mod search_trace;
#[cfg(feature = "search")]
mod semantic_query_json;
#[cfg(feature = "search")]
mod semantic_query_projection;
#[cfg(feature = "search")]
mod semantic_read_json;
#[cfg(feature = "search")]
mod semantic_search_json;
#[cfg(feature = "search")]
mod semantic_search_json_canonical;
#[cfg(feature = "search")]
mod semantic_search_json_fields;
#[cfg(feature = "search")]
mod semantic_search_synthesis_json;
#[cfg(feature = "search")]
mod semantic_syntax_refs;
mod tree_sitter_query;
mod tree_sitter_query_locator;
mod tree_sitter_query_packet;
mod tree_sitter_query_projection;

pub use runner::run_cli_from_env;
