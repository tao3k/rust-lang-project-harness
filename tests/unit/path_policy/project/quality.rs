use std::fs;
use std::path::Path;

use rust_lang_project_harness::{
    default_rust_harness_config, run_rust_project_harness, run_rust_project_harness_with_config,
};
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, has_rule, write_manifest};

#[test]
fn weak_advice_allow_explanation_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "weak-advice-allowance");
    let config = default_rust_harness_config()
        .with_cargo_check_advice_allow_explanation("temporary migration");

    let report = run_rust_project_harness_with_config(root, &config).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R017");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("cargo_check_advice_allow_explanation"),
        "{:?}",
        findings[0]
    );
}

#[test]
fn structured_advice_allow_explanation_is_allowed() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "structured-advice-allowance");
    let config = default_rust_harness_config().with_cargo_check_advice_allow_explanation(
        "scope=test build.rs gates; owner=test-harness; finding_category=advisory \
         project-policy migrations; why_safe_now=warnings stay visible in harness output; \
         cleanup_trigger=remove once strict downstream gate is enabled",
    );

    let report = run_rust_project_harness_with_config(root, &config).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R017"),
        "{:?}",
        report.findings
    );
}

#[test]
fn fake_cargo_package_identity_fallback_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "fake-cargo-identity");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn cargo_package_name_from_env() -> String {\n    \
         std::env::var(\"CARGO_PKG_NAME\")\n        \
         .unwrap_or_else(|_| String::from(\"unknown-cargo-package\"))\n}\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R018"), "{:?}", report.findings);
}

#[test]
fn redundant_workspace_member_build_gate_alias_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "redundant-build-gate-alias");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn assert_member_build_gate_from_env() {\n    \
         assert_member_harness_build_gate_from_env();\n}\n\n\
         pub fn assert_member_harness_build_gate_from_env() {}\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R019"), "{:?}", report.findings);
}

fn write_minimal_project(root: &Path, name: &str) {
    write_manifest(root, name);
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
}
