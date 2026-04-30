use std::fs;
use std::path::Path;

use tempfile::TempDir;
use xiuxian_harness_rust_lang_project::{render_rust_project_harness, run_rust_project_harness};

#[test]
fn agent_r001_public_module_intent_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r001-intent");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "pub fn public_api() {}\n").expect("write lib");

    assert_agent_snapshot(root, "AGENT-R001", 1, "agent_r001_public_module_intent");
}

#[test]
fn agent_r002_public_item_doc_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r002-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub struct MissingDoc;\n",
    )
    .expect("write owned");

    assert_agent_snapshot(root, "AGENT-R002", 1, "agent_r002_public_item_doc");
}

#[test]
fn agent_r003_repeated_namespace_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r003-namespace");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
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
    .expect("write repeated");

    assert_agent_snapshot(root, "AGENT-R003", 1, "agent_r003_repeated_namespace");
}

#[test]
fn agent_r004_public_name_conflict_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r004-conflict");
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

    assert_agent_snapshot(root, "AGENT-R004", 2, "agent_r004_public_name_conflict");
}

#[test]
fn agent_r005_facade_reexports_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r005-reexports");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), facade_reexports()).expect("write lib");

    assert_agent_snapshot(root, "AGENT-R005", 1, "agent_r005_facade_reexports");
}

#[test]
fn agent_r006_generic_public_module_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r006-public-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n/// Shared utility bucket.\npub mod utils;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/utils.rs"), "//! Utility bucket.\n").expect("write utils");

    assert_agent_snapshot(root, "AGENT-R006", 1, "agent_r006_generic_public_module");
}

#[test]
fn agent_r007_generic_module_path_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r007-path");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(root.join("src/helpers.rs"), "//! Helper bucket.\n").expect("write helpers");

    assert_agent_snapshot(root, "AGENT-R007", 1, "agent_r007_generic_module_path");
}

#[test]
fn agent_r008_branch_intent_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r008-branch");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "mod parse;\nmod render;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");
    fs::write(root.join("src/domain/render.rs"), "//! Render leaf.\n").expect("write render");

    assert_agent_snapshot(root, "AGENT-R008", 1, "agent_r008_branch_intent");
}

fn assert_agent_snapshot(
    root: &Path,
    rule_id: &str,
    expected_count: usize,
    snapshot_name: &'static str,
) {
    let mut report = run_rust_project_harness(root).expect("run project harness");
    report.findings.retain(|finding| finding.rule_id == rule_id);
    assert_eq!(
        report.findings.len(),
        expected_count,
        "expected {expected_count} {rule_id} finding(s), got {:?}",
        report.findings
    );
    let rendered = normalize_temp_root(&render_rust_project_harness(&report), root);
    insta::assert_snapshot!(snapshot_name, rendered);
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn facade_reexports() -> String {
    let mut source = String::from("//! Test crate.\n");
    for index in 0..29 {
        source.push_str(&format!("pub use owner_{index}::Thing{index};\n"));
    }
    source
}
