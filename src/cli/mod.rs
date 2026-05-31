//! Command-line execution for the Rust project harness binary.

mod agent_registry;
mod runner;
#[cfg(feature = "search")]
mod search_trace;
#[cfg(feature = "search")]
mod semantic_search_json;

pub use runner::run_cli_from_env;
