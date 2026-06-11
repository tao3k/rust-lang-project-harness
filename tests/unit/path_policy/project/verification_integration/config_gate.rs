use std::fs;

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, default_rust_harness_config,
    render_rust_project_harness, run_rust_project_harness, run_rust_project_harness_with_config,
};
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn cargo_test_gate_requires_explicit_verification_config() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "empty-verification-config");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R016");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("without explicit verification config"),
        "{:?}",
        findings[0]
    );
    assert!(
        findings_for_rule(&report, "RUST-PROJ-R011").is_empty(),
        "{:?}",
        report.findings
    );

    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-PROJ-R016");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("cargo_test_gate_requires_verification_config", rendered);
}

#[test]
fn configured_cargo_test_gate_clears_verification_config_warning() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "configured-verification-config");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!(config = {\n    rust_lang_project_harness::default_rust_harness_config()\n});\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness_with_config(root, &configured_no_external_tasks_config())
        .expect("run project harness");

    assert!(
        findings_for_rule(&report, "RUST-PROJ-R016").is_empty(),
        "{:?}",
        report.findings
    );
}

#[test]
fn positional_config_gate_still_requires_named_verification_config() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "positional-verification-config");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!(rust_lang_project_harness::default_rust_harness_config());\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R016");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
}

#[test]
fn advice_allow_gate_still_requires_explicit_verification_config() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "advice-allow-without-config");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!(advice = allow);\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R016");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    let allow_findings = findings_for_rule(&report, "RUST-PROJ-R015");
    assert_eq!(allow_findings.len(), 1, "{:?}", report.findings);
}

#[test]
fn advice_allow_with_config_still_requires_allow_explanation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "advice-allow-with-config");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!(advice = allow, config = {\n    rust_lang_project_harness::default_rust_harness_config()\n});\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness_with_config(root, &configured_no_external_tasks_config())
        .expect("run project harness");

    assert!(
        findings_for_rule(&report, "RUST-PROJ-R016").is_empty(),
        "{:?}",
        report.findings
    );
    let findings = findings_for_rule(&report, "RUST-PROJ-R015");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("advice allowance but no explicit allow explanation"),
        "{:?}",
        findings[0]
    );

    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-PROJ-R015");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("advice_allow_requires_explanation", rendered);
}

#[test]
fn advice_allow_with_explanation_clears_allow_warning() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "advice-allow-with-explanation");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!(advice = allow, config = {\n    rust_lang_project_harness::default_rust_harness_config()\n});\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness_with_config(
        root,
        &configured_no_external_tasks_config().with_cargo_test_advice_allow_explanation(
            "legacy fixture allows advisory output during migration",
        ),
    )
    .expect("run project harness");

    assert!(
        findings_for_rule(&report, "RUST-PROJ-R016").is_empty(),
        "{:?}",
        report.findings
    );
    assert!(
        findings_for_rule(&report, "RUST-PROJ-R015").is_empty(),
        "{:?}",
        report.findings
    );
}

fn configured_no_external_tasks_config() -> rust_lang_project_harness::RustHarnessConfig {
    default_rust_harness_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/lib.rs", [RustOwnerResponsibility::PublicApi])
            .without_verification_tasks()
            .with_rationale(
                "this fixture only verifies legacy cargo-test gate configuration plumbing",
            ),
    )
}

fn normalize_temp_root(rendered: &str, root: &std::path::Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
