use std::fs;
use std::process::Command;

#[test]
fn query_code_uses_workspace_option_and_rejects_trailing_root() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    fs::write(root.path().join("src/lib.rs"), "pub fn target() {}\n").expect("write fixture");

    let current = Command::new(bin)
        .args([
            "query",
            "--from-hook",
            "direct-source-read",
            "--selector",
            "src/lib.rs:1:1",
            "--workspace",
        ])
        .arg(root.path())
        .arg("--code")
        .current_dir(root.path())
        .output()
        .expect("run current query command");

    assert!(
        current.status.success(),
        "current command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&current.stdout),
        String::from_utf8_lossy(&current.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&current.stdout),
        "pub fn target() {}\n"
    );

    let stale = Command::new(bin)
        .args([
            "query",
            "--from-hook",
            "direct-source-read",
            "--selector",
            "src/lib.rs:1:1",
            "--code",
        ])
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run stale query command");

    assert!(
        !stale.status.success(),
        "stale command unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&stale.stderr)
            .contains("query --code does not accept a trailing PROJECT_ROOT"),
        "stderr={}",
        String::from_utf8_lossy(&stale.stderr)
    );

    let tree_sitter_current = Command::new(bin)
        .args([
            "query",
            "--catalog",
            "declarations",
            "--selector",
            "src/lib.rs:1:1",
            "--workspace",
        ])
        .arg(root.path())
        .arg("--code")
        .current_dir(root.path())
        .output()
        .expect("run current tree-sitter query command");

    assert!(
        tree_sitter_current.status.success(),
        "current tree-sitter command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&tree_sitter_current.stdout),
        String::from_utf8_lossy(&tree_sitter_current.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&tree_sitter_current.stdout),
        "pub fn target() {}\n"
    );

    let tree_sitter_stale = Command::new(bin)
        .args([
            "query",
            "--catalog",
            "declarations",
            "--selector",
            "src/lib.rs:1:1",
            "--code",
        ])
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run stale tree-sitter query command");

    assert!(
        !tree_sitter_stale.status.success(),
        "stale tree-sitter command unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&tree_sitter_stale.stderr)
            .contains("query --code does not accept a trailing PROJECT_ROOT"),
        "stderr={}",
        String::from_utf8_lossy(&tree_sitter_stale.stderr)
    );
}
