//! Agent hook bridge into the root semantic-agent-hook runtime.

mod bridge;

pub(super) use bridge::{print_agent_guide, run_agent_guard, run_agent_hook};
