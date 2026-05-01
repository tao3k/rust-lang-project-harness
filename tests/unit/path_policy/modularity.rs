use std::fs;

use rust_lang_project_harness::run_rust_project_harness;
use tempfile::TempDir;

use super::support::{findings_for_rule, has_rule, private_implementation_pile, write_manifest};

#[test]
fn deep_relative_import_policy_uses_native_use_trees() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "deep-relative-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\npub use super::{super::MissingOwner, sibling::Thing};\n",
    )
    .expect("write domain");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    let source_line = findings[0].source_line.as_deref().expect("source line");
    assert!(source_line.contains("super::{super::MissingOwner, sibling::Thing}"));
}

#[test]
fn deep_relative_import_policy_ignores_comments_and_strings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "deep-relative-comment");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\n// use super::super::MissingOwner;\nconst HELP: &str = \"use super::super::MissingOwner\";\n",
    )
    .expect("write domain");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R003"), "{:?}", report.findings);
}

#[test]
fn glob_import_policy_flags_native_use_tree_globs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "glob-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nuse crate::prelude::*;\nuse super::{sibling::Thing, traits::*};\n",
    )
    .expect("write domain");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R010");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(
        findings
            .iter()
            .all(|finding| finding.summary.contains("glob import"))
    );
}

#[test]
fn glob_import_policy_ignores_comments_and_strings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "glob-comment");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\n// use crate::prelude::*;\nconst HELP: &str = \"use crate::prelude::*\";\n",
    )
    .expect("write domain");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R010"), "{:?}", report.findings);
}

#[test]
fn interface_mod_policy_rejects_inline_module_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "inline-mod-implementation");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain interface.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write mod");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R001");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .source_line
            .as_deref()
            .is_some_and(|line| line.contains("mod leaf"))
    );
}

#[test]
fn source_bloat_policy_reports_private_implementation_pile() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "private-implementation-pile");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod pile;\n").expect("write lib");
    fs::write(root.join("src/pile.rs"), private_implementation_pile()).expect("write pile");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("top-level implementation items")
    );
}

#[test]
fn module_source_layout_policy_rejects_file_and_mod_rs_pair() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "shadowed-module-source");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "//! Domain owner.\n").expect("write file form");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain directory owner.\n",
    )
    .expect("write mod form");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R007");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/domain.rs"));
    assert!(findings[0].summary.contains("src/domain/mod.rs"));
}

#[test]
fn inline_source_module_policy_rejects_reasoning_tree_collapse() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "inline-source-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain owner.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write domain");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R008");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .source_line
            .as_deref()
            .is_some_and(|line| line.contains("mod leaf"))
    );
}

#[test]
fn orphan_source_file_policy_rejects_unreachable_module_file() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "orphan-source-file");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(root.join("src/forgotten.rs"), "//! Forgotten owner.\n").expect("write orphan");

    let report = run_rust_project_harness(root).expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R009");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("src/forgotten.rs"))
    );
}

#[test]
fn orphan_policy_does_not_treat_latest_feature_as_cfg_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-feature-latest-reachability");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(feature = \"latest\")]\nmod optional;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/optional.rs"), "//! Optional owner.\n").expect("write optional");

    let report = run_rust_project_harness(root).expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R009"), "{:?}", report.findings);
}
