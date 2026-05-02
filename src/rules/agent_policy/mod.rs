//! Agent-oriented Rust policy rules.

mod dependency_graph;
mod pack;
mod source_surface;

pub(crate) use pack::evaluate;
pub use pack::rust_agent_policy_rules;
use pack::{
    AGENT_R001, AGENT_R002, AGENT_R003, AGENT_R004, AGENT_R005, AGENT_R006, AGENT_R007, AGENT_R008,
    AGENT_R009, AGENT_R010, AGENT_R011, AGENT_R012, AGENT_R013, AGENT_R014,
};
