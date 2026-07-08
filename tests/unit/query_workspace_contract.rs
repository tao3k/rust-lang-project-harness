use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn query_code_rejects_trailing_root_and_catalog_accepts_positional_workspace() {
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
            .contains("query does not accept positional WORKSPACE"),
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

    let tree_sitter_positional = Command::new(bin)
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
        .expect("run positional tree-sitter query command");

    assert!(
        tree_sitter_positional.status.success(),
        "positional tree-sitter command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&tree_sitter_positional.stdout),
        String::from_utf8_lossy(&tree_sitter_positional.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&tree_sitter_positional.stdout),
        "pub fn target() {}\n"
    );
}

#[test]
fn query_names_only_rejects_workspace_term_discovery() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    fs::write(root.path().join("src/lib.rs"), "pub fn run_install() {}\n").expect("write fixture");

    let output = Command::new(bin)
        .args(["query", "--term", "run_install", "--workspace"])
        .arg(root.path())
        .arg("--names-only")
        .current_dir(root.path())
        .output()
        .expect("run ambiguous query command");

    assert!(
        !output.status.success(),
        "ambiguous command unexpectedly succeeded: stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("query --names-only requires an owner selector"),
        "stderr={stderr}"
    );
    assert!(
        stderr.contains("workspace term discovery is hook-managed"),
        "stderr={stderr}"
    );
    assert!(
        !stderr.contains("asp rust search lexical"),
        "stderr={stderr}"
    );
}

#[test]
fn query_exact_owner_names_only_does_not_scan_workspace_context() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    fs::create_dir_all(root.path().join("tests")).expect("create tests");
    fs::write(
        root.path().join("src/lib.rs"),
        "pub fn target_symbol() {}\n",
    )
    .expect("write source fixture");
    for index in 0..800 {
        fs::write(
            root.path()
                .join("tests")
                .join(format!("fixture_{index}.rs")),
            format!(
                "#[test]\nfn generated_test_{index}() {{\n    assert_eq!({}, {});\n}}\n",
                index, index
            ),
        )
        .expect("write test fixture");
    }

    let query_args = [
        "query",
        "--selector",
        "src/lib.rs",
        "--term",
        "target_symbol",
        "--names-only",
        "--workspace",
    ];
    let warmup = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("warm exact owner names-only query");
    assert!(
        warmup.status.success(),
        "warm exact owner names-only failed: stdout={} stderr={}",
        String::from_utf8_lossy(&warmup.stdout),
        String::from_utf8_lossy(&warmup.stderr)
    );

    let started_at = Instant::now();
    let output = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run exact owner names-only query");
    let elapsed = started_at.elapsed();

    assert!(
        output.status.success(),
        "exact owner names-only failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "exact owner names-only scanned too much workspace context: elapsed={elapsed:?}; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("target_symbol"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
