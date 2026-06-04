use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::support::{run_cli_with_stdin, write_clean_source, write_manifest};

fn ast_patch_cli_test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn cli_ast_patch_dry_run_returns_provider_unsupported_operation_receipt() {
    let _guard = ast_patch_cli_test_guard();
    let packet = json!({
        "target": { "ownerPath": "src/lib.rs", "locator": "src/lib.rs#fn:demo", "read": "src/lib.rs:1:4" },
        "operation": { "op": "remove_statement", "snippet": "return;" }
    })
    .to_string();

    let output = run_cli_with_stdin(["ast-patch", "dry-run", "--packet", "-", "."], &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["capability"], "provider-ast-dry-run");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(receipt["supportedOperations"], json!(["replace_item"]));
    assert_eq!(receipt["operation"], "remove_statement");
    assert_eq!(receipt["target"]["read"], "src/lib.rs:1:4");

    let next = receipt["next"].as_str().unwrap();
    assert!(next.contains("replace_item"));
    assert!(next.contains("asp ast-patch template --language rust"));
    assert!(next.contains("--owner src/lib.rs"));
    assert!(next.contains("--op replace_item"));
    assert!(next.contains("asp rust ast-patch dry-run"));
    assert!(next.contains("asp rust ast-patch apply"));
    assert!(next.contains("asp rust check --changed"));
}

#[test]
fn cli_ast_patch_dry_run_verifies_replace_item_without_mutating_file() {
    let _guard = ast_patch_cli_test_guard();
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("fn demo() -> usize { 42 }\n");
    let before = fs::read_to_string(root.path().join("src/lib.rs")).expect("read before");

    let output = run_ast_patch_with_packet("dry-run", root.path(), &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "verified");
    assert_eq!(receipt["mode"], "dry-run");
    assert_eq!(receipt["capability"], "provider-ast-dry-run");
    assert_eq!(receipt["mutationAvailable"], false);
    assert_eq!(receipt["supportedOperations"], json!(["replace_item"]));
    assert_eq!(receipt["mechanicalEditPlan"]["kind"], "provider-dry-run");
    assert_eq!(
        receipt["mechanicalEditPlan"]["requiresCodexApplyPatch"],
        false
    );
    assert_receipt_verification_contains(&receipt, "target-item-parsed");
    assert_receipt_verification_contains(&receipt, "file-reparsed");

    let after = fs::read_to_string(root.path().join("src/lib.rs")).expect("read after");
    assert_eq!(before, after);
}

#[test]
fn cli_ast_patch_apply_replaces_item_and_formats_file() {
    let _guard = ast_patch_cli_test_guard();
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("pub fn demo() -> usize { 2 }");
    let output = run_ast_patch_with_packet("apply", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "successful ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let source = fs::read_to_string(root.path().join("src/lib.rs")).expect("source");
    assert!(source.contains("pub fn demo() -> usize {\n    2\n}"));
    assert!(!source.contains("usize { 1 }"));
}

#[test]
fn cli_ast_patch_apply_does_not_write_when_rustfmt_fails() {
    let _guard = ast_patch_cli_test_guard();
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("pub fn demo() -> usize { 2 }");
    let source_path = root.path().join("src/lib.rs");
    let before = fs::read_to_string(&source_path).expect("read before");

    let output = run_ast_patch_with_packet_and_path("apply", root.path(), &packet, "");
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt JSON");

    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["failureKind"], "rustfmt-error");
    assert_receipt_verification_contains(&receipt, "file-reparsed");

    let after = fs::read_to_string(source_path).expect("read after");
    assert_eq!(before, after);
}

#[test]
fn cli_ast_patch_apply_rejects_append_to_existing_module_file() {
    let _guard = ast_patch_cli_test_guard();
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-append-existing");
    fs::create_dir_all(root.path().join("src")).expect("mkdir src");
    fs::write(root.path().join("src/mod.rs"), "").expect("write mod");
    let packet = json!({
        "target": { "ownerPath": "src/mod.rs", "locator": "src/mod.rs#module-root", "read": "src/mod.rs:1:1" },
        "operation": { "op": "append_to_block", "snippet": "pub mod read;\n", "maxEdits": 1 }
    })
    .to_string();

    let output = run_ast_patch_with_packet("apply", root.path(), &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed", "{receipt}");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(receipt["operation"], "append_to_block");
    assert_eq!(receipt["supportedOperations"], json!(["replace_item"]));
    assert!(receipt["mechanicalEditPlan"].is_null());
    let source = fs::read_to_string(root.path().join("src/mod.rs")).expect("source");
    assert_eq!(source, "");
}

#[test]
fn cli_ast_patch_apply_rejects_append_to_missing_file() {
    let _guard = ast_patch_cli_test_guard();
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-append-missing-file");
    fs::create_dir_all(root.path().join("src")).expect("mkdir src");
    let packet = json!({
        "target": { "ownerPath": "src/generated.rs", "locator": "src/generated.rs#module-root", "read": "src/generated.rs:1:1" },
        "operation": { "op": "append_to_block", "snippet": "pub fn generated_marker() -> usize { 1 }\n", "maxEdits": 1 }
    })
    .to_string();

    let output = run_ast_patch_with_packet("apply", root.path(), &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed", "{receipt}");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(receipt["operation"], "append_to_block");
    assert_eq!(receipt["supportedOperations"], json!(["replace_item"]));
    assert!(receipt["mechanicalEditPlan"].is_null());
    assert!(!root.path().join("src/generated.rs").exists());
}

fn write_replace_item_fixture() -> TempDir {
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-fixture");
    write_clean_source(root.path());
    fs::write(
        root.path().join("src/lib.rs"),
        "pub fn demo() -> usize {\n    1\n}\n",
    )
    .expect("write lib");
    root
}

fn replace_item_packet(snippet: &str) -> String {
    json!({
        "target": {
            "ownerPath": "src/lib.rs",
            "locator": "src/lib.rs#fn:demo",
            "read": "src/lib.rs:1:3",
            "itemName": "demo",
            "itemKind": "fn"
        },
        "operation": {
            "op": "replace_item",
            "snippet": snippet,
            "expectedSnippet": "pub fn demo",
            "maxEdits": 1
        }
    })
    .to_string()
}

fn run_ast_patch_with_packet(
    mode: &str,
    root: &std::path::Path,
    packet: &str,
) -> std::process::Output {
    run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        packet,
    )
}

fn run_ast_patch_with_packet_and_path(
    mode: &str,
    root: &std::path::Path,
    packet: &str,
    path_env: &str,
) -> std::process::Output {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args([
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ])
        .env("PATH", path_env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(packet.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait cli")
}

fn assert_receipt_verification_contains(receipt: &Value, expected: &str) {
    let verification = receipt["verification"].as_array().expect("verification");
    assert!(
        verification.iter().any(|value| value == expected),
        "missing verification {expected}: {verification:?}"
    );
}
