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
            "rust://src/lib.rs#item/function/target",
            "--workspace",
        ])
        .arg(root.path())
        .arg("--code")
        .arg("--json")
        .current_dir(root.path())
        .output()
        .expect("run current query command");

    assert!(
        current.status.success(),
        "current command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&current.stdout),
        String::from_utf8_lossy(&current.stderr)
    );
    let packet = serde_json::from_slice::<serde_json::Value>(&current.stdout)
        .expect("exact source query should emit typed JSON");
    assert_eq!(packet["schemaId"], "asp.exact-source-query-result.v1");
    assert_eq!(packet["code"], "pub fn target() {}");
    assert_eq!(
        packet["resolutionEvidence"]["snapshotRoot"],
        packet["sourceSnapshot"]["rootDigest"]
    );
    assert!(
        packet["sourceSnapshot"]["rootDigest"]
            .as_str()
            .is_some_and(|digest| !digest.is_empty())
    );
    assert!(
        packet["resolutionEvidence"]["parserArtifactDigest"]
            .as_str()
            .is_some_and(|digest| !digest.is_empty())
    );
    let state = packet["resolutionEvidence"]["state"].as_str();
    let authority = packet["resolutionEvidence"]["authority"].as_str();
    assert!(matches!(
        (state, authority),
        (Some("live-hit"), Some("live-parser"))
            | (Some("artifact-cache-hit"), Some("content-cache"))
    ));

    let stale = Command::new(bin)
        .args([
            "query",
            "--from-hook",
            "direct-source-read",
            "--selector",
            "rust://src/lib.rs#item/function/target",
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
        stderr.contains("query requires an exact --selector"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("asp rust search owner"), "stderr={stderr}");
}

#[test]
fn search_exact_owner_names_only_does_not_scan_workspace_context() {
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
        "search",
        "owner",
        "src/lib.rs",
        "items",
        "--query",
        "target_symbol",
        "--names-only",
        "--workspace",
    ];
    let warmup = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("warm exact owner names-only search");
    assert!(
        warmup.status.success(),
        "warm exact owner names-only search failed: stdout={} stderr={}",
        String::from_utf8_lossy(&warmup.stdout),
        String::from_utf8_lossy(&warmup.stderr)
    );

    let started_at = Instant::now();
    let output = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run exact owner names-only search");
    let elapsed = started_at.elapsed();

    assert!(
        output.status.success(),
        "exact owner names-only search failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "exact owner names-only search scanned too much workspace context: elapsed={elapsed:?}; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("target_symbol"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
