//! Command-line execution for the Rust project harness binary.

mod agent_registry;
mod behavior_snapshot;
mod determinism_readiness;
mod dev_command_log;
mod evidence_graph;
mod execution_receipt;
mod formal_proof_pilot;
mod query;
mod review_packet;
mod runner;
#[cfg(feature = "search")]
mod search_output;
#[cfg(feature = "search")]
mod search_plan;
#[cfg(feature = "search")]
mod search_trace;
#[cfg(feature = "search")]
mod semantic_query_json;
#[cfg(feature = "search")]
mod semantic_read_json;
mod semantic_search_finder_json;
#[cfg(feature = "search")]
mod semantic_search_json;
#[cfg(feature = "search")]
mod semantic_search_json_fields;

pub use runner::run_cli_from_env;
