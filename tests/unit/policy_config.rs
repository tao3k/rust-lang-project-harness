use std::fs;
use std::path::Path;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

#[test]
fn layout_policy_requires_explanations_for_root_file_exceptions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::write(
        root.join("tests/custom_gate.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n",
    )
    .expect("write custom gate");

    write_policy(
        root,
        "[tests]\nallowed_root_files = [\n  { name = \"custom_gate.rs\", explanation = \"\" },\n]\n",
    );
    let report = run_rust_project_harness(root).expect("run project harness");
    assert!(has_rule(&report, "RUST-PROJ-R001"));

    write_policy(
        root,
        "[tests]\nallowed_root_files = [\n  { name = \"custom_gate.rs\", explanation = \"explicit harness aggregate\" },\n]\n",
    );
    let report = run_rust_project_harness(root).expect("run project harness");
    assert!(!has_rule(&report, "RUST-PROJ-R001"));
}

#[test]
fn layout_policy_requires_explanations_for_directory_exceptions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::create_dir(root.join("tests/contract")).expect("create contract tests");
    fs::write(root.join("tests/contract/fixtures.rs"), "fn helper() {}\n")
        .expect("write contract fixture");

    write_policy(
        root,
        "[tests]\nallowed_directories = [\n  { name = \"contract\", explanation = \"\" },\n]\n",
    );
    let report = run_rust_project_harness(root).expect("run project harness");
    assert!(has_rule(&report, "RUST-PROJ-R002"));

    write_policy(
        root,
        "[tests]\nallowed_directories = [\n  { name = \"contract\", explanation = \"contract fixtures mounted by a root gate\" },\n]\n",
    );
    let report = run_rust_project_harness(root).expect("run project harness");
    assert!(!has_rule(&report, "RUST-PROJ-R002"));
}

fn write_minimal_project(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"policy-config\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
}

fn write_policy(root: &Path, content: &str) {
    fs::write(root.join("tests/rust-project-harness-rules.toml"), content)
        .expect("write policy config");
}

fn has_rule(report: &rust_lang_project_harness::RustHarnessReport, rule_id: &str) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.rule_id == rule_id)
}
