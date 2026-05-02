use std::fs;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

use super::support::{findings_for_rule, has_rule, write_manifest};

#[path = "project/verification_integration.rs"]
mod verification_integration;

#[test]
fn source_test_policy_does_not_treat_latest_feature_as_cfg_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-feature-latest-source");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(feature = \"latest\")]\nmod optional;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/optional.rs"), "//! Optional owner.\n").expect("write optional");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R003"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-PROJ-R004"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_accepts_embedded_cargo_test_gate_macro() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "embedded-gate-target");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write root test target");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R006"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_accepts_library_cargo_test_gate_macro() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "embedded-lib-gate-targets");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nrust_lang_project_harness::rust_project_harness_cargo_test_gate!();\n",
    )
    .expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "//! Thin root target.\n#[path = \"unit/suite.rs\"]\nmod suite;\n",
    )
    .expect("write root test target");
    fs::create_dir_all(root.join("tests/unit")).expect("create test suite dir");
    fs::write(
        root.join("tests/unit/suite.rs"),
        "#[test]\nfn suite_runs() {}\n",
    )
    .expect("write suite");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R006"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_ignores_comment_mentions_of_harness_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "comment-mention-root-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "//! Mentioning rust_project_harness_gate!() here is not a gate.\nconst NOTE: &str = \"run_rust_project_harness(.)\";\n",
    )
    .expect("write root test target");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R006"), "{:?}", report.findings);
}

#[test]
fn library_target_requires_cargo_test_gate_when_harness_is_dev_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "missing-embedded-lib-gate");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"missing-embedded-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn library_target_ignores_comment_mentions_of_embedded_cargo_test_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"comment-mention-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Mentioning rust_project_harness_cargo_test_gate!() here is not a gate.\nconst NOTE: &str = \"rust_project_harness_cargo_test_gate!()\";\n",
    )
    .expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn library_target_accepts_embedded_cargo_test_gate() {
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

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R009"),
        "{:?}",
        report.findings
    );
}

#[test]
fn manifest_comment_does_not_enable_library_harness_policy() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"manifest-comment-only\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n# rust-lang-project-harness is mentioned in prose, not dependencies.\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-PROJ-R009"),
        "{:?}",
        report.findings
    );
}

#[test]
fn manifest_package_field_uses_the_canonical_harness_identity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"manifest-package-field\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies.local-harness]\npackage = \"rust-lang-project-harness\"\npath = \".\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn target_dependency_table_uses_canonical_harness_identity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"target-dependency-table\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[target.'cfg(unix)'.dev-dependencies]\nrust-lang-project-harness = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(has_rule(&report, "RUST-PROJ-R009"), "{:?}", report.findings);
}

#[test]
fn large_unit_test_leaf_is_reported_from_parser_source_metrics() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "large-unit-test-leaf");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create tests/unit");
    fs::write(root.join("tests/unit/large.rs"), large_unit_test_leaf()).expect("write large leaf");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-PROJ-R005");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("large.rs"));
}

fn large_unit_test_leaf() -> String {
    let mut source = String::new();
    for index in 0..8 {
        source.push_str("#[test]\n");
        source.push_str(&format!("fn large_test_{index}() {{\n"));
        for value in 0..34 {
            source.push_str(&format!("    let value_{value} = {value};\n"));
            source.push_str(&format!("    assert_eq!(value_{value}, {value});\n"));
        }
        source.push_str("}\n");
    }
    source
}
