//! Scenario benchmark contracts for Rust harness fixtures.

mod contract_gate;
mod core;

pub use core::{
    RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration, RustScenarioBenchmarkError,
    RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkMemoryBytes,
    RustScenarioBenchmarkReceipt, RustScenarioBenchmarkRequirement, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
    assert_rule_fixture_scenario_benchmarks, discover_required_rust_scenario_benchmarks,
    render_rust_scenario_benchmark_gate_failure, render_rust_scenario_benchmark_snapshot,
    render_rust_scenario_benchmark_suite_snapshot, validate_required_rust_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};
