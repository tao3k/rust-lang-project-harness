#![deny(dead_code)]

#[cfg(feature = "cli")]
#[path = "unit/cli/mod.rs"]
mod cli;

#[path = "unit/public_api/mod.rs"]
mod public_api;

#[path = "unit/policy_contract.rs"]
mod policy_contract;

#[path = "unit/policy_config.rs"]
mod policy_config;

#[path = "unit/path_policy.rs"]
mod path_policy;

#[path = "unit/rule_catalog.rs"]
mod rule_catalog;

#[path = "unit/invariant_catalog.rs"]
mod invariant_catalog;

#[path = "unit/render_snapshot.rs"]
mod render_snapshot;

#[path = "unit/mod_policy_snapshot.rs"]
mod mod_policy_snapshot;

#[path = "unit/agent_policy_snapshot.rs"]
mod agent_policy_snapshot;

#[path = "unit/agent_quality_signal_snapshot.rs"]
mod agent_quality_signal_snapshot;

#[path = "unit/agent_reasoning_snapshot.rs"]
mod agent_reasoning_snapshot;

#[path = "unit/verification/mod.rs"]
mod verification;

#[path = "unit/runner_config/mod.rs"]
mod runner_config;

#[path = "unit/sample_project.rs"]
mod sample_project;

#[path = "unit/search.rs"]
mod search;
