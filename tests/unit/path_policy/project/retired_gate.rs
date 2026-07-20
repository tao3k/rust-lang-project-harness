use std::fs;
use std::path::Path;

use rust_lang_project_harness::{render_rust_project_harness, run_rust_project_harness_for_scope};
use tempfile::TempDir;

use crate::path_policy::support::has_rule;

#[test]
fn retired_root_cargo_test_gate_reports_migration_warning() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"retired-root-cargo-test-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n",
    )
    .expect("write root test target");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-006");
    assert_eq!(focused_report.findings.len(), 1, "{:?}", report.findings);
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!(
        "retired_root_cargo_test_gate_reports_migration_warning",
        rendered
    );
}

#[test]
fn retired_source_cargo_test_gate_reports_migration_warning() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"embedded-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );

    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-009");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!(
        "retired_source_cargo_test_gate_reports_migration_warning",
        rendered
    );
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
