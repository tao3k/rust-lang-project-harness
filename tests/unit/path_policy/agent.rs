use std::fs;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

use super::support::{findings_for_rule, write_manifest};

#[test]
fn repeated_namespace_policy_includes_file_stems() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "repeated-file-stem");
    fs::create_dir_all(root.join("src/domain")).expect("create source namespace");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain branch.\nmod domain;\n",
    )
    .expect("write branch");
    fs::write(
        root.join("src/domain/domain.rs"),
        "//! Repeated domain namespace.\nfn local() {}\n",
    )
    .expect("write repeated namespace module");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/domain/domain"));
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("src/domain/domain.rs"))
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn branch_module_without_intent_doc_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "branch-without-intent");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "mod parse;\nmod render;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");
    fs::write(root.join("src/domain/render.rs"), "//! Render leaf.\n").expect("write render");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R008");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("2 resolved child edges"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn branch_intent_counts_resolved_reasoning_tree_edges() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "branch-resolved-children");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "mod parse;\nmod missing;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R008");
    assert!(findings.is_empty(), "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_source_module_paths_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-source-path");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(root.join("src/helpers.rs"), "//! Helper bucket.\n").expect("write helpers");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R007");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`helpers`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_public_module_names_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-public-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n/// Shared utility bucket.\npub mod utils;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/utils.rs"), "//! Utility bucket.\n").expect("write utils");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R006");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("generic public module `utils`")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_doc_policy_ignores_comment_text_that_mentions_docs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-doc-comment-text");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "//! Owner module.\n// /// Pretend doc text.\npub struct MissingDoc;\n",
    )
    .expect("write owner");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`MissingDoc`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn module_intent_policy_uses_native_inner_doc_attributes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "module-intent-native-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "// ! Pretend module doc text.\n/// Owner handle.\npub struct Owner;\n",
    )
    .expect("write owner");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R001");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/owner.rs"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn repeated_namespace_policy_covers_default_test_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "repeated-test-root");
    fs::create_dir_all(root.join("tests/unit/unit")).expect("create repeated test namespace");
    fs::write(
        root.join("tests/unit/unit/helper.rs"),
        "fn helper_fixture() {}\n",
    )
    .expect("write repeated test helper");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "AGENT-R003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("tests/unit/unit"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn duplicated_public_names_are_reported_as_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "duplicated-public-names");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\n/// Alpha handle.\npub struct Handle;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\n/// Beta handle.\npub struct Handle;\n",
    )
    .expect("write beta");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert_eq!(findings_for_rule(&report, "AGENT-R004").len(), 2);
    assert!(report.is_clean(), "{:?}", report.findings);
}
