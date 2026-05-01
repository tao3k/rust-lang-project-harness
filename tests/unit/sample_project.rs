use std::fs;
use std::path::Path;

use rust_lang_project_harness::{render_rust_project_harness, run_rust_project_harness};
use tempfile::TempDir;

#[test]
fn project_runner_reports_blocking_policy_and_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn public_api() {}\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/test_root.rs"),
        "#[test] fn root_test() {}\n",
    )
    .expect("write root test");

    let report = run_rust_project_harness(root).expect("run project harness");
    let rendered = normalize_temp_root(&render_rust_project_harness(&report), root);

    assert!(!report.is_clean());
    assert!(rendered.contains("RUST-PROJ-R001"));
    assert!(rendered.contains("RUST-PROJ-R003"));
    assert!(rendered.contains("AGENT-R001"));
    assert!(rendered.contains("Help: tests/test_root.rs is a root-level test file"));
    assert!(rendered.contains("Contract: Move root-level test files under tests/unit"));
    assert!(!rendered.contains("Required:"));
    insta::assert_snapshot!("sample_project_blocking_and_agent_advice", rendered);
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    rendered.replace(&root.display().to_string(), "$TEMP")
}
