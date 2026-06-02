use std::fs;

use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture};

#[test]
fn cli_query_hook_line_range_code_outputs_local_window() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"line-range\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write manifest");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn first() {}\npub fn second() {}\npub fn third() {}\n",
    )
    .expect("write source");
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:2:2".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
    assert!(!stdout.contains("[read-plan]"), "{stdout}");
    assert!(stdout.contains("pub fn second()"), "{stdout}");
    assert!(!stdout.contains("pub fn first()"), "{stdout}");
}

#[test]
fn cli_query_hook_wide_line_range_code_returns_read_plan_without_source() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "src/lib.rs:1:80".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.starts_with("[read-plan] "), "{stdout}");
    assert!(stdout.contains("mode=range-outline"), "{stdout}");
    assert!(stdout.contains("code=false"), "{stdout}");
    assert!(stdout.contains("reason=wide-selector"), "{stdout}");
    assert!(stdout.contains("requested=1:80"), "{stdout}");
    assert!(!stdout.contains("pub fn load()"), "{stdout}");
    assert!(!stdout.contains("[search-owner]"), "{stdout}");
}
