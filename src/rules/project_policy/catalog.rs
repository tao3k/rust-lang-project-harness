//! Project-policy rule catalog.

use std::collections::BTreeMap;

use crate::rules::labels;
use crate::{RustDiagnosticSeverity, RustHarnessRule};

use super::{
    PACK_ID, RUST_PROJ_R001, RUST_PROJ_R002, RUST_PROJ_R003, RUST_PROJ_R004, RUST_PROJ_R005,
    RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008, RUST_PROJ_R009, RUST_PROJ_R010, RUST_PROJ_R011,
    RUST_PROJ_R012, RUST_PROJ_R013, RUST_PROJ_R014, RUST_PROJ_R015,
};

pub(super) fn rules_by_id() -> BTreeMap<&'static str, RustHarnessRule> {
    [
        RustHarnessRule::new(
            RUST_PROJ_R001,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test file lacks explicit harness role",
            "Move root-level test files under tests/unit or tests/integration, or justify an explicit harness entry point in tests/rust-project-harness-rules.toml.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R002,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Unexpected tests directory",
            "Keep only standard suite directories directly under tests, or document the exception in tests/rust-project-harness-rules.toml.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R003,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Inline source test module",
            "Keep test bodies out of src files; mount source-backed unit tests from tests/unit with #[path].",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R004,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "External test mount is missing or misplaced",
            "External source-backed tests must resolve to existing files under tests/unit.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R005,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Large test leaf should split",
            "Split oversized test leaves into folder-first suites with focused modules.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R006,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Cargo test target does not mount the harness gate",
            "Mount rust_project_harness_cargo_test_gate!(config = ...) in the library test target, or mount rust_project_harness_gate!() in standalone Cargo test targets.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R007,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test target contains implementation",
            "Keep root Cargo test targets as thin aggregates with external module mounts only.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R008,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test module mount is implicit or misplaced",
            "Root Cargo test targets must mount external modules with explicit #[path] attributes pointing under an allowed tests suite directory.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R009,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Library target does not mount the cargo-test harness gate",
            "Mount rust_project_harness_cargo_test_gate!(config = ...) from a #[cfg(test)] source module so cargo test --lib executes project and verification policy.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R010,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Performance verification skill lacks Cargo bench target",
            "When a Rust-native performance skill is configured, expose a runnable harness=false [[bench]] target so cargo test can remind agents when benchmark wiring is missing.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R011,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Harness gate uses empty verification config",
            "Mount harness gates with explicit verification profile hints, task suppressions, or skill bindings so cargo test tells agents which verification duties apply.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R012,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Build-time harness gate is incomplete",
            "When a harness-enabled package has a root build.rs or a harness build-dependency, mount the build-time harness gate so filtered cargo test runs cannot bypass project policy.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R013,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Custom harness scope path lacks explanation",
            "Custom source or test scope paths must be added with an explicit explanation so agents cannot shrink the harness to avoid existing policy debt.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R014,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Cargo-backed harness scope reduction lacks explanation",
            "Cargo-backed source or test scopes that exist in the package must stay covered or be removed with an explicit explanation so agents cannot skip existing source or test debt.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R015,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Harness advice allowance lacks explanation",
            "Cargo-test advice allowance must include an explicit explanation so agents cannot silence advisory policy just to pass the harness.",
            labels("project-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
