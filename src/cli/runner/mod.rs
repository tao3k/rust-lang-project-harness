//! CLI runner module interface.

mod dispatch;
mod options;

use dispatch::{AgentOptions, CliOptions, ResolvedCheckTarget};

pub use dispatch::run_cli_from_env;
