//! Scenario benchmark contracts for Rust harness fixtures.

mod contract;
mod contract_gate;
mod core;
mod discovery;
mod render;
mod types;

pub use contract::{
    RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration, RustScenarioBenchmarkMemoryBytes,
};
pub use core::{
    assert_rule_fixture_scenario_benchmarks, validate_required_rust_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};
pub use discovery::discover_required_rust_scenario_benchmarks;
pub use render::{
    render_rust_scenario_benchmark_gate_failure, render_rust_scenario_benchmark_snapshot,
    render_rust_scenario_benchmark_suite_snapshot,
};
pub use types::{
    RustScenarioBenchmarkError, RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkReceipt,
    RustScenarioBenchmarkRequirement, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
};
