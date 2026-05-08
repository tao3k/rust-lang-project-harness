//! Agent-oriented Rust policy rules.

mod algorithm_shape;
mod api_shape;
mod data_shape;
mod dependency_graph;
mod pack;
mod source_surface;

pub(crate) use pack::evaluate;
pub use pack::rust_agent_policy_rules;
use pack::{
    AGENT_R001, AGENT_R002, AGENT_R003, AGENT_R004, AGENT_R005, AGENT_R006, AGENT_R007, AGENT_R008,
    AGENT_R009, AGENT_R010, AGENT_R011, AGENT_R012, AGENT_R013, AGENT_R014, AGENT_R015, AGENT_R016,
    AGENT_R017, AGENT_R018, AGENT_R019, AGENT_R020, AGENT_R021, AGENT_R022, AGENT_R023, AGENT_R024,
    AGENT_R025, AGENT_R026, AGENT_R027, AGENT_R028,
};
