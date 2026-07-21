//! CLI runner module interface.

mod dispatch;
mod options;
mod tree_sitter_query;

use dispatch::{AgentOptions, CliOptions, ResolvedCheckTarget};

pub use dispatch::run_cli_from_env;
