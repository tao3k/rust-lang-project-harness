//! Project-policy rule catalog.

use std::collections::BTreeMap;

use crate::rules::labels;
use crate::{RustDiagnosticSeverity, RustHarnessRule};

use super::{
    PACK_ID, RUST_PROJ_R001, RUST_PROJ_R002, RUST_PROJ_R003, RUST_PROJ_R004, RUST_PROJ_R005,
    RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008,
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
            "Mount rust_project_harness_gate!() in Cargo test targets so narrow test runs execute project policy.",
            labels("project-policy"),
        ),
        RustHarnessRule::new(
            RUST_PROJ_R007,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test target contains implementation",
            "Keep root Cargo test targets as thin harness aggregates with harness gate calls and external module mounts only.",
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
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
