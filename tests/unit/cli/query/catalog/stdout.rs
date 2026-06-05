use tempfile::TempDir;

use super::function_name_query_args;
use crate::cli::support::run_cli;

#[test]
fn tree_sitter_query_locator_output_names_captures_without_artifact_scope() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn exposed() -> usize {\n    1\n}\n\nstruct Hidden;\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(root, &[]));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "src/lib.rs:1:3\nexposed\n");
    assert!(!stdout.contains("pub fn exposed()"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
    assert!(!stdout.contains("clientDbHint="));
    assert!(!stdout.contains("matches=0"));
}

#[test]
fn tree_sitter_query_locator_output_can_be_filtered_by_term() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    1\n}\n\npub fn beta_target() -> usize {\n    2\n}\n",
    )
    .expect("fixture");

    let output = run_cli(function_name_query_args(root, &["--term", "beta"]));
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8(output.stdout).expect("compact output is UTF-8");
    assert_eq!(stdout, "src/lib.rs:5:7\nbeta_target\n");
    assert!(!stdout.contains("name=alpha"));
    assert!(!stdout.contains("pub fn"));
    assert!(!stdout.contains("----"));
    assert!(!stdout.contains("|syntax-query"));
    assert!(!stdout.contains("|syntax-capture"));
    assert!(!stdout.contains("artifactId="));
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
