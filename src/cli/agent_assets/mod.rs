//! Rust agent profile publication for the root semantic-agent-hook runtime.

mod codex;

pub(super) use codex::{
    AgentConfigScope, ensure_rust_agent_profile_registry, install_agent_assets, print_agent_doctor,
    run_semantic_agent_hook,
};
