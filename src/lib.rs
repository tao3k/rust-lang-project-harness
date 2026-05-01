//! Project-level Rust language harness for policy gates and agent advice.
//!
//! The crate provides library APIs for scanning Rust projects, returning
//! deterministic findings, rendering compact diagnostics, and mounting a
//! reusable Cargo test gate.

mod agent_snapshot;
mod cli;
mod discovery;
mod macros;
mod model;
mod parser;
mod render;
mod rules;
mod runner;
mod self_policy;
mod verification;

#[cfg(test)]
#[path = "../tests/unit/discovery.rs"]
mod discovery_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_source_path.rs"]
mod parser_source_path_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_reasoning_tree.rs"]
mod parser_reasoning_tree_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax.rs"]
mod parser_native_syntax_tests;

pub use agent_snapshot::{
    render_rust_project_harness_agent_snapshot,
    render_rust_project_harness_agent_snapshot_with_config,
};
pub use cli::run_cli_from_env;
pub use discovery::{DEFAULT_IGNORED_DIR_NAMES, discover_rust_files, rust_project_harness_scope};
pub use model::{
    RulePackDescriptor, RustDiagnosticSeverity, RustHarnessConfig, RustHarnessFinding,
    RustHarnessReport, RustHarnessRule, RustModuleReport, RustProjectHarnessScope, RustRulePack,
    SourceLocation,
};
pub use render::{
    render_rust_project_harness, render_rust_project_harness_advice,
    render_rust_project_harness_json, render_rust_project_harness_with_options,
};
pub use rules::{
    rust_agent_policy_rules, rust_modularity_rules, rust_project_policy_rules,
    rust_rule_pack_descriptors, rust_syntax_rules,
};
pub use runner::{
    assert_rust_lang_harness_clean, assert_rust_project_harness_clean, default_rust_harness_config,
    run_rust_lang_harness, run_rust_lang_harness_with_config, run_rust_project_harness,
    run_rust_project_harness_with_config,
};
pub use verification::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationReceiptStatus, RustVerificationRequirement, RustVerificationResolutionNote,
    RustVerificationTask, RustVerificationTaskKind, RustVerificationTaskState,
    RustVerificationWaiver, plan_rust_project_verification,
    plan_rust_project_verification_with_config, plan_rust_project_verification_with_policy,
    render_rust_verification_plan, render_rust_verification_plan_json,
};
