use tempfile::TempDir;

use crate::cli::query::catalog::function_name_query_args;
use crate::cli::support::run_cli;

#[test]
fn tree_sitter_query_code_rejects_multiple_matches() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(root, &["--code"]));
    assert!(!output.status.success(), "{output:?}");

    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("query --code matched 2 items; add an exact --selector"),
        "{stderr}"
    );
}

#[test]
fn tree_sitter_query_exact_selector_code_output_is_plain_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn locate_me() -> usize {\n    7\n}\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(
        root,
        &["--selector", "src/lib.rs:1:3", "--code"],
    ));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "pub fn locate_me() -> usize {\n    7\n}\n");
}

#[test]
fn tree_sitter_query_range_locator_can_drive_exact_code_output() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(
        root,
        &["--selector", "src/lib.rs:5:7", "--code"],
    ));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "pub fn beta_target() -> usize {\n    2\n}\n");
}
